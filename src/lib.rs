mod into_response;
mod request;
mod request_parser;
mod response;
mod routing;
mod status;

use into_response::{convert, IntoResponse};
use request::Request;
use request_parser::RequestParser;
use response::Response;
use routing::{delete, get, patch, post, put, static_files, Route, Router};
use status::Status;

use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{SocketAddr, TcpListener},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use pyo3::{prelude::*, types::PyDict};

#[pyclass]
struct HttpServer {
    addr: SocketAddr,
    routers: Vec<Router>,
    app_data: Option<Py<PyAny>>,
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
        })
    }

    fn app_data(&mut self, app_data: Py<PyAny>) {
        self.app_data = Some(app_data)
    }

    fn attach(&mut self, router: PyRef<'_, Router>) {
        self.routers.push(router.clone());
    }

    fn run(&self, py: Python<'_>) -> PyResult<()> {
        let running = Arc::new(AtomicBool::new(true));
        let r = running.clone();

        let addr = self.addr;

        ctrlc::set_handler(move || {
            println!("\nReceived Ctrl+C! Shutting Down...");
            r.store(false, Ordering::SeqCst);
            _ = std::net::TcpStream::connect(addr);
        })
        .ok();

        let listener = TcpListener::bind(addr)?;
        println!("Listening on {}", addr);

        while running.load(Ordering::SeqCst) {
            let (mut socket, _) = listener.accept()?;

            let mut request_data = Vec::new();
            let mut buffer = [0; 8192];

            loop {
                match socket.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(n) => {
                        request_data.extend_from_slice(&buffer[..n]);
                        if let Ok(request_str) = String::from_utf8(request_data.clone()) {
                            if request_str.contains("\r\n\r\n") {
                                break;
                            }
                        }
                    }
                    Err(_) => {
                        break;
                    }
                }
            }
            if let Ok(request_str) = String::from_utf8(request_data) {
                if let Some(ref request) = RequestParser::parse(&request_str) {
                    let mut response = Status::NOT_FOUND().into_response();

                    for router in &self.routers {
                        for route in &router.routes {
                            if route.method == request.method {
                                if let Some(params) = route.match_path(&request.url) {
                                    response = match self
                                        .process_response(py, router, route, request, params)
                                    {
                                        Ok(response) => response,
                                        Err(e) => Status::INTERNAL_SERVER_ERROR()
                                            .into_response()
                                            .body(e.to_string()),
                                    };
                                    break;
                                }
                            }
                        }
                    }

                    socket.write_all(response.to_string().as_bytes())?;
                    socket.flush()?;
                }
            }
        }

        Ok(())
    }
}

impl HttpServer {
    fn process_response(
        &self,
        py: Python<'_>,
        router: &Router,
        route: &Arc<Route>,
        request: &Request,
        params: HashMap<String, String>,
    ) -> PyResult<Response> {
        let kwargs = PyDict::new(py);

        kwargs.set_item("request", request.clone())?;

        if let (Some(app_data), true) = (
            self.app_data.as_ref(),
            route.args.contains(&"app_data".to_string()),
        ) {
            kwargs.set_item("app_data", app_data)?;
        }

        for (key, value) in &params {
            kwargs.set_item(key, value)?;
        }

        let mut body_param_name = None;

        for key in route.args.clone() {
            if key != "app_data" && !params.contains_key(&key) {
                body_param_name = Some(key);
                break;
            }
        }

        if let (Some(ref body_name), Ok(ref body)) = (body_param_name, request.json(py)) {
            kwargs.set_item(body_name, body)?;
        }

        if let Some(middleware) = &router.middleware {
            kwargs.set_item("next", route.handler.clone_ref(py))?;
            let result = middleware.call(py, (), Some(&kwargs))?;
            return convert(result, py);
        }

        kwargs.del_item("request")?;

        let result = route.handler.call(py, (), Some(&kwargs))?;
        convert(result, py)
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
