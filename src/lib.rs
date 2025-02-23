mod into_response;
mod request;
mod request_parser;
mod response;
mod routing;
mod status;

use into_response::{convert_to_response, IntoResponse};
use matchit::Match;
use request::Request;
use request_parser::RequestParser;
use response::Response;
use routing::{delete, get, patch, post, put, static_files, Route, Router};
use status::Status;

use std::{
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

            let mut buffer = [0; 1024];
            let n = socket.read(&mut buffer)?;
            let request_str = String::from_utf8_lossy(&buffer[..n]);

            if let Ok(ref request) = RequestParser::parse(&request_str) {
                let mut response = Status::NOT_FOUND().into_response();

                for router in &self.routers {
                    if let Ok(route) = &router.router.at(&request.url) {
                        response = match self.process_response(py, router, route, request) {
                            Ok(response) => response,
                            Err(e) => Status::INTERNAL_SERVER_ERROR()
                                .into_response()
                                .body(e.to_string()),
                        };
                        break;
                    }
                }

                socket.write_all(response.to_string().as_bytes())?;
                socket.flush()?;
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
            kwargs.set_item("app_data", app_data)?;
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
