mod routing;
mod status;

use routing::{delete, get, patch, post, put, Router};
use status::{IsHttpError, Status};

use std::{
    fmt,
    io::{Read, Write},
    net::{SocketAddr, TcpListener},
};

use pyo3::{
    prelude::*,
    types::{PyDict, PyTuple},
};

#[derive(Clone)]
#[pyclass]
struct Response {
    status: Status,
    content_type: String,
    body: String,
}

#[pymethods]
impl Response {
    #[new]
    fn new(status: PyRef<'_, Status>, body: PyObject, py: Python<'_>) -> PyResult<Self> {
        let (content_type, body_str) = if let Ok(dict) = body.downcast_bound::<PyDict>(py) {
            let json_mod = PyModule::import(py, "json")?;
            let json_str = json_mod
                .call_method("dumps", (dict,), None)?
                .extract::<String>()?;
            ("application/json", json_str)
        } else {
            let repr = body.bind(py).repr()?.extract::<String>()?;
            ("text/plain", repr)
        };

        Ok(Self {
            status: status.clone(),
            content_type: content_type.to_string(),
            body: body_str.to_string(),
        })
    }
}

impl fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n{}",
            self.status.code(),
            self.status.reason(),
            self.content_type,
            self.body.len(),
            self.body
        )
    }
}

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
            addr: SocketAddr::new(ip.parse().unwrap(), port),
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
            let request = String::from_utf8_lossy(&buffer[..n]);

            let request_line = request.lines().next().unwrap_or("");
            let parts: Vec<&str> = request_line.split_whitespace().collect();
            if parts.len() < 3 {
                continue;
            }

            let method = parts[0].to_string();
            let path = parts[1].to_string();

            let mut response = Response {
                status: Status(404),
                content_type: "text/plain".to_string(),
                body: "NotFound".to_string(),
            };

            for router in &self.routers {
                for route in &router.routes {
                    if route.method == method {
                        if let Some(params) = route.match_path(&path) {
                            let params_tuple: Vec<&str> = route
                                .param_names
                                .iter()
                                .filter_map(|name| params.get(name).map(|s| s.as_str()))
                                .collect();

                            let handler = &route.handler;
                            let args = PyTuple::new(py, params_tuple)?;

                            match self.process_response(py, router, handler, &args) {
                                Ok(resp) => response = resp,
                                Err(e) => {
                                    response = Response {
                                        status: Status(500),
                                        content_type: "text/plain".to_string(),
                                        body: e.to_string(),
                                    }
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
        handler: &Py<PyAny>,
        args: &Bound<'_, PyTuple>,
    ) -> PyResult<Response> {
        for middleware in &router.middlewares {
            let result = middleware.call(py, args, None)?;
            let response = result.extract::<PyRef<'_, Response>>(py)?;
            if response.status.code().is_http_error() {
                return Ok(response.clone());
            }
        }

        let result = handler.call(py, args, None)?;
        let response = result.extract::<PyRef<'_, Response>>(py)?;
        Ok(response.clone())
    }
}

#[pymodule]
fn oxhttp(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<HttpServer>()?;
    m.add_class::<Router>()?;
    m.add_class::<Status>()?;
    m.add_class::<Response>()?;
    m.add_function(wrap_pyfunction!(get, m)?)?;
    m.add_function(wrap_pyfunction!(post, m)?)?;
    m.add_function(wrap_pyfunction!(delete, m)?)?;
    m.add_function(wrap_pyfunction!(patch, m)?)?;
    m.add_function(wrap_pyfunction!(put, m)?)?;
    Ok(())
}
