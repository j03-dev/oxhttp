mod into_response;
mod request;
mod response;
mod routing;
mod status;

use into_response::IntoResponse;
use request::Request;
use response::Response;
use routing::{delete, get, patch, post, put, Route, Router};
use status::Status;

use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{SocketAddr, TcpListener},
    sync::Arc,
};

use pyo3::{prelude::*, types::PyDict};

#[pyclass]
struct HttpServer {
    addr: SocketAddr,
    routers: Vec<Router>,
}

#[pymethods]
impl HttpServer {
    #[new]
    fn new(addr: (String, u16)) -> PyResult<Self> {
        let (ip, port) = addr;
        Ok(Self {
            addr: SocketAddr::new(ip.parse()?, port),
            routers: Vec::new(),
        })
    }

    fn attach(&mut self, router: PyRef<'_, Router>) {
        self.routers.push(router.clone());
    }

    fn run(&self, py: Python<'_>) -> PyResult<()> {
        let listener = TcpListener::bind(self.addr)?;
        println!("Listening on {}", self.addr);

        loop {
            let (mut socket, _) = listener.accept()?;
            let mut buffer = [0; 1024];
            let n = socket.read(&mut buffer)?;
            let request_str = String::from_utf8_lossy(&buffer[..n]);

            let request_line = request_str.lines().next().unwrap_or("");
            let parts: Vec<&str> = request_line.split_whitespace().collect();
            if parts.len() < 3 {
                continue;
            }

            let method = parts[0].to_string();
            let path = parts[1].to_string();

            let mut headers = HashMap::new();
            let mut body = String::new();
            let mut is_body = false;

            for line in request_str.lines().skip(1) {
                if is_body {
                    body.push_str(line);
                    body.push('\n');
                } else if line.is_empty() {
                    is_body = true;
                }
                let header_parts: Vec<&str> = line.split(": ").collect();
                if header_parts.len() == 2 {
                    headers.insert(header_parts[0].to_string(), header_parts[1].to_string());
                }
            }

            let request = Request::new(method.clone(), path.clone(), headers, body);

            let mut response = Status::NOT_FOUND().into_response();

            for router in &self.routers {
                for route in &router.routes {
                    if route.method == method {
                        if let Some(params) = route.match_path(&path) {
                            match self.process_response(py, router, route, request.clone(), params)
                            {
                                Ok(resp) => response = resp,
                                Err(e) => {
                                    response = Status::INTERNAL_SERVER_ERROR()
                                        .into_response()
                                        .body(e.to_string())
                                }
                            }
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

impl HttpServer {
    fn process_response(
        &self,
        py: Python<'_>,
        router: &Router,
        route: &Arc<Route>,
        request: Request,
        params: HashMap<String, String>,
    ) -> PyResult<Response> {
        let kwargs = PyDict::new(py);

        kwargs.set_item("request", request.clone())?;
        kwargs.set_item("next", route.handler.clone_ref(py))?;

        for (key, value) in &params {
            kwargs.set_item(key, value)?;
        }

        let mut body_param_name = None;

        for key in route.args.clone() {
            if !params.contains_key(&key) {
                body_param_name = Some(key);
                break;
            }
        }

        if let (Some(body_name), Ok(body)) = (body_param_name, request.json(py)) {
            kwargs.set_item(body_name.clone(), body.clone_ref(py))?;
        }

        for middleware in &router.middlewares {
            let result = middleware.call(py, (), Some(&kwargs))?;
            to_response!(result, py);
        }

        kwargs.del_item("request")?;
        kwargs.del_item("next")?;

        let result = route.handler.call(py, (), Some(&kwargs))?;
        to_response!(result, py);
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
    Ok(())
}
