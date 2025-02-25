mod into_response;
mod request;
mod response;
mod routing;
mod status;

use http_body_util::{BodyExt, Full};
use hyper::body::{Bytes, Incoming};
use into_response::{convert_to_response, IntoResponse};
use pyo3::exceptions::PyException;
use request::Request;
use response::Response;
use routing::{delete, get, patch, post, put, static_files, Route, Router};
use status::Status;

use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request as HyperRequest, Response as HyperResponse};
use hyper_util::rt::TokioIo;

use matchit::Match;
use tokio::net::TcpListener;
use tokio::sync::mpsc::{channel, Sender};
use tokio::sync::Semaphore;

use std::mem::transmute;
use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use pyo3::{prelude::*, types::PyDict};

type MatchitRoute = &'static Match<'static, 'static, &'static Route>;
type BoxResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

struct ProcessRequest {
    request: Request,
    router: Router,
    route: MatchitRoute,
    response_sender: Sender<Response>,
}

#[derive(Clone)]
#[pyclass]
struct HttpServer {
    addr: SocketAddr,
    routers: Vec<Router>,
    app_data: Option<Arc<Py<PyAny>>>,
    max_connections: Arc<Semaphore>,
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

    #[pyo3(signature=(max_connections = 100, channel_capacity = 100))]
    fn config(&mut self, max_connections: usize, channel_capacity: usize) -> PyResult<()> {
        self.max_connections = Arc::new(Semaphore::new(max_connections));
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
        })
        .ok();

        let listener = TcpListener::bind(addr).await?;
        println!("Listening on {}", addr);

        let routers = self.routers.clone();
        let running_clone = running.clone();
        let sender = request_sender.clone();
        let max_connections = self.max_connections.clone();
        let channel_capacity = self.channel_capacity;

        tokio::spawn(async move {
            while running_clone.load(Ordering::SeqCst) {
                let permit = max_connections.clone().acquire_owned().await.unwrap();
                let (stream, _) = listener.accept().await.unwrap();
                let io = TokioIo::new(stream);
                let sender = sender.clone();
                let routers = routers.clone();

                tokio::spawn(async move {
                    let _permit = permit;
                    if let Err(err) = http1::Builder::new()
                        .serve_connection(
                            io,
                            service_fn(move |req| {
                                let sender = sender.clone();
                                let routers = routers.clone();

                                async move {
                                    let request = convert_hyper_request(req).await.unwrap();

                                    for router in &routers {
                                        if let Some(method) = router.routes.get(&request.method) {
                                            if let Ok(route) = method.at(&request.url) {
                                                let (response_sender, mut respond_receive) =
                                                    channel(channel_capacity);

                                                let route: MatchitRoute =
                                                    unsafe { transmute(&route) };

                                                let process_request = ProcessRequest {
                                                    request: request.clone(),
                                                    router: router.clone(),
                                                    route,
                                                    response_sender,
                                                };

                                                if sender.send(process_request).await.is_ok() {
                                                    if let Some(response) =
                                                        respond_receive.recv().await
                                                    {
                                                        return convert_to_hyper_response(response);
                                                    }
                                                }
                                                break;
                                            }
                                        }
                                    }

                                    convert_to_hyper_response(Status::FOUND().into_response())
                                }
                            }),
                        )
                        .await
                    {
                        return Err(PyException::new_err(format!(
                            "Error serving connectio {err}"
                        )));
                    }

                    Ok::<(), PyErr>(())
                });
            }
        });

        while running.load(Ordering::SeqCst) {
            if let Ok(process_request) = request_receiver.try_recv() {
                let response = match self
                    .process_response(
                        &process_request.router,
                        process_request.route,
                        &process_request.request,
                    )
                    .await
                {
                    Ok(response) => response,
                    Err(e) => Status::INTERNAL_SERVER_ERROR()
                        .into_response()
                        .body(e.to_string()),
                };

                _ = process_request.response_sender.send(response).await;
            }
        }

        Ok(())
    }

    async fn process_response(
        &self,
        router: &Router,
        matchit_route: MatchitRoute,
        request: &Request,
    ) -> PyResult<Response> {
        Python::with_gil(|py| {
            let kwargs = PyDict::new(py);

            let route = matchit_route.value.clone();
            let params = matchit_route.params.clone();

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

            let result = if let Some(middleware) = &router.middleware {
                kwargs.set_item("request", request.clone())?;
                kwargs.set_item("next", route.handler.clone_ref(py))?;
                middleware.call(py, (), Some(&kwargs))?
            } else {
                route.handler.call(py, (), Some(&kwargs))?
            };

            convert_to_response(result, py)
        })
    }
}

async fn convert_hyper_request(req: HyperRequest<Incoming>) -> BoxResult<Request> {
    let method = req.method().to_string();
    let uri = req.uri().to_string();

    let mut headers = std::collections::HashMap::new();
    for (key, value) in req.headers() {
        headers.insert(
            key.to_string(),
            value.to_str().unwrap_or_default().to_string(),
        );
    }

    let mut request = Request::new(method, uri, headers);

    let body_bytes = req.collect().await?.to_bytes();
    let body = String::from_utf8_lossy(&body_bytes).to_string();
    if !body.is_empty() {
        request.set_body(body);
    }

    Ok(request)
}

fn convert_to_hyper_response(
    response: Response,
) -> Result<HyperResponse<Full<Bytes>>, hyper::http::Error> {
    HyperResponse::builder()
        .status(response.status.code())
        .header("Content-Type", response.content_type)
        .body(Full::new(Bytes::from(response.body)))
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
