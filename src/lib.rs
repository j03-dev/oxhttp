mod into_response;
mod request;
mod request_parser;
mod response;
mod routing;
mod status;

use into_response::{convert_to_response, IntoResponse};
use matchit::Match;
use pyo3::exceptions::PyException;
use request::Request;
use request_parser::RequestParser;
use response::Response;
use routing::{delete, get, patch, post, put, static_files, Route, Router};
use status::Status;

use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc::{channel, Sender};
use tokio::sync::Semaphore;
use tokio::{io::AsyncReadExt, net::TcpListener};

use std::mem::transmute;
use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use pyo3::{prelude::*, types::PyDict};

type MatchedRoute = &'static Match<'static, 'static, &'static Route>;

struct ProcessRequest {
    request: Request,
    router: Router,
    matched_route: MatchedRoute,
    response_sender: Sender<Response>,
}

#[derive(Clone)]
#[pyclass]
struct HttpServer {
    addr: SocketAddr,
    routers: Vec<Router>,
    app_data: Option<Arc<Py<PyAny>>>,
    max_connections: Arc<Semaphore>,
    buffer_size: usize,
    channel_capacity: usize,
}

#[pymethods]
impl HttpServer {
    #[new]
    fn new(addr: (String, u16)) -> PyResult<Self> {
        let (ip, port) = addr;
        Ok(Self {
            addr: SocketAddr::new(ip.parse()?, port),
            routers: Vec::new(),
            app_data: None,
            max_connections: Arc::new(Semaphore::new(100)),
            buffer_size: 16384,
            channel_capacity: 100,
        })
    }

    fn app_data(&mut self, app_data: Py<PyAny>) {
        self.app_data = Some(Arc::new(app_data))
    }

    fn attach(&mut self, router: PyRef<'_, Router>) {
        self.routers.push(router.clone());
    }

    fn run(&self) -> PyResult<()> {
        let runtime = tokio::runtime::Runtime::new()?;
        runtime.block_on(async move { self.run_server().await })?;
        Ok(())
    }

    #[pyo3(signature=(max_connections = 100, buffer_size=16384, channel_capacity = 100))]
    fn config(
        &mut self,
        max_connections: usize,
        buffer_size: usize,
        channel_capacity: usize,
    ) -> PyResult<()> {
        self.max_connections = Arc::new(Semaphore::new(max_connections));
        self.buffer_size = buffer_size;
        self.channel_capacity = channel_capacity;
        Ok(())
    }
}

impl HttpServer {
    async fn run_server(&self) -> PyResult<()> {
        let running = Arc::new(AtomicBool::new(true));
        let r = running.clone();
        let addr = self.addr;
        let channel_capacity = self.channel_capacity;

        let (request_sender, mut request_receiver) = channel::<ProcessRequest>(channel_capacity);

        ctrlc::set_handler(move || {
            println!("\nReceived Ctrl+C! Shutting Down...");
            r.store(false, Ordering::SeqCst);
            _ = std::net::TcpStream::connect(addr);
        })
        .ok();

        let listener = TcpListener::bind(addr).await?;
        println!("Listening on {}", addr);

        let routers = self.routers.clone();
        let running_clone = running.clone();
        let sender = request_sender.clone();
        let max_connections = self.max_connections.clone();
        let buffer_size = self.buffer_size;
        let channel_capacity = self.channel_capacity;

        tokio::spawn(async move {
            while running_clone.load(Ordering::SeqCst) {
                let permit = max_connections
                    .clone()
                    .acquire_owned()
                    .await
                    .map_err(|err| PyException::new_err(err.to_string()))?;
                let (mut socket, _) = listener.accept().await?;
                let sender = sender.clone();
                let routers = routers.clone();

                tokio::spawn(async move {
                    let _permit = permit;
                    let mut buffer = vec![0; buffer_size];
                    let n = socket.read(&mut buffer).await?;
                    let request_str = String::from_utf8_lossy(&buffer[..n]);

                    if let Ok(request) = RequestParser::parse(&request_str) {
                        for router in &routers {
                            if let Some(method) = router.routes.get(&request.method) {
                                if let Ok(route) = method.at(&request.url) {
                                    let (response_sender, mut response_receive) =
                                        channel(channel_capacity);

                                    let matched_route: MatchedRoute = unsafe { transmute(&route) };

                                    let process_request = ProcessRequest {
                                        request: request.clone(),
                                        router: router.clone(),
                                        matched_route,
                                        response_sender,
                                    };

                                    if sender.send(process_request).await.is_ok() {
                                        if let Some(response) = response_receive.recv().await {
                                            socket
                                                .write_all(response.to_string().as_bytes())
                                                .await?;
                                            socket.flush().await?;
                                        }
                                    }

                                    break;
                                }
                            }
                        }
                        let response = Status::NOT_FOUND().into_response().to_string();
                        socket.write_all(response.as_bytes()).await?;
                    }
                    Ok::<(), PyErr>(())
                });
            }
            Ok::<(), PyErr>(())
        });

        while running.load(Ordering::SeqCst) {
            if let Ok(process_request) = request_receiver.try_recv() {
                let response = Python::with_gil(|py| {
                    match self.process_response(
                        py,
                        &process_request.router,
                        process_request.matched_route,
                        &process_request.request,
                    ) {
                        Ok(response) => response,
                        Err(e) => Status::INTERNAL_SERVER_ERROR()
                            .into_response()
                            .body(e.to_string()),
                    }
                });

                _ = process_request.response_sender.send(response).await;
            }
        }

        Ok(())
    }

    fn process_response(
        &self,
        py: Python<'_>,
        router: &Router,
        match_route: &Match<'_, '_, &Route>,
        request: &Request,
    ) -> PyResult<Response> {
        let kwargs = PyDict::new(py);

        let route = match_route.value.clone();
        let params = match_route.params.clone();

        if let (Some(app_data), true) = (
            self.app_data.as_ref(),
            route.args.contains(&"app_data".to_string()),
        ) {
            kwargs.set_item("app_data", app_data.clone_ref(py))?;
        }

        for (key, value) in params.iter() {
            kwargs.set_item(key, value)?;
        }

        let mut body_param_name = None;

        for key in route.args.clone() {
            if key != "app_data"
                && params
                    .iter()
                    .filter(|(k, _)| *k == key)
                    .collect::<Vec<_>>()
                    .is_empty()
            {
                body_param_name = Some(key);
                break;
            }
        }

        if let (Some(ref body_name), Ok(ref body)) = (body_param_name, request.json(py)) {
            kwargs.set_item(body_name, body)?;
        }

        if let Some(middleware) = &router.middleware {
            kwargs.set_item("request", request.clone())?;
            kwargs.set_item("next", route.handler.clone_ref(py))?;
            let result = middleware.call(py, (), Some(&kwargs))?;
            return convert_to_response(result, py);
        }

        let result = route.handler.call(py, (), Some(&kwargs))?;
        convert_to_response(result, py)
    }
}

#[pymodule]
fn oxhttp(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<HttpServer>()?;
    m.add_class::<Router>()?;
    m.add_class::<Status>()?;
    m.add_class::<Response>()?;
    m.add_class::<Request>()?;
    m.add_function(wrap_pyfunction!(get, m)?)?;
    m.add_function(wrap_pyfunction!(post, m)?)?;
    m.add_function(wrap_pyfunction!(delete, m)?)?;
    m.add_function(wrap_pyfunction!(patch, m)?)?;
    m.add_function(wrap_pyfunction!(put, m)?)?;
    m.add_function(wrap_pyfunction!(static_files, m)?)?;

    Ok(())
}
