mod routing;

use routing::Router;

use std::{
    fmt,
    io::{Read, Write},
    net::{SocketAddr, TcpListener},
};

use pyo3::{
    prelude::*,
    types::{PyDict, PyTuple},
};

enum Status {
    Ok,
    Created,
    NotFound,
    InternalServerError,
    Unknow,
}

impl Status {
    fn new(code: u16) -> Self {
        match code {
            200 => Self::Ok,
            201 => Self::Created,
            404 => Self::NotFound,
            500 => Self::InternalServerError,
            _ => Self::Unknow,
        }
    }

    fn code(&self) -> u16 {
        match self {
            Self::Ok => 200,
            Self::Created => 201,
            Self::NotFound => 404,
            Self::InternalServerError => 500,
            Self::Unknow => 0,
        }
    }

    fn reason(&self) -> &str {
        match self {
            Self::Ok => "Ok",
            Self::Created => "Created",
            Self::NotFound => "Not Found",
            Self::InternalServerError => "Internal Server Error",
            Self::Unknow => "Unknow",
        }
    }
}

struct Response {
    status: Status,
    content_type: String,
    body: String,
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
    router: Router,
}

#[pymethods]
impl HttpServer {
    #[new]
    fn new(addr: (String, u16)) -> PyResult<Self> {
        let (ip, port) = addr;
        Ok(Self {
            addr: SocketAddr::new(ip.parse().unwrap(), port),
            router: Router::new(),
        })
    }

    fn attach(&mut self, router: PyRef<'_, Router>) -> PyResult<()> {
        self.router.routes.extend(router.routes.clone());
        Ok(())
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
                status: Status::NotFound,
                content_type: "text/plain".to_string(),
                body: "NotFound".to_string(),
            };

            for route in &self.router.routes {
                if route.method == method {
                    if let Some(params) = route.match_path(&path) {
                        let params_tuple: Vec<&str> = route
                            .param_names
                            .iter()
                            .filter_map(|name| params.get(name).map(|s| s.as_str()))
                            .collect();

                        let handler = &route.handler;
                        let args = PyTuple::new(py, params_tuple)?;
                        match self.process_response(py, handler, &args) {
                            Ok(resp) => response = resp,
                            Err(e) => {
                                response = Response {
                                    status: Status::InternalServerError,
                                    content_type: "text/plain".to_string(),
                                    body: e.to_string(),
                                }
                            }
                        }
                        break;
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
        handler: &Py<PyAny>,
        args: &Bound<'_, PyTuple>,
    ) -> PyResult<Response> {
        let result = handler.call(py, args, None)?;

        // Handle different response formats
        let (body, status) = if let Ok((body, status)) = result.extract::<(PyObject, u16)>(py) {
            (body, status)
        } else {
            (result.into_pyobject(py)?.into(), 200)
        };

        // Process body content
        let (content_type, body_str) = if let Ok(s) = body.extract::<String>(py) {
            ("text/plain", s)
        } else if let Ok(dict) = body.downcast_bound::<PyDict>(py) {
            let json_mod = PyModule::import(py, "json")?;
            let json_str = json_mod
                .call_method("dumps", (dict,), None)?
                .extract::<String>()?;
            ("application/json", json_str)
        } else {
            let repr = body.bind(py).repr()?.extract::<String>()?;
            ("text/plain", repr)
        };

        Ok(Response {
            status: Status::new(status),
            content_type: content_type.to_string(),
            body: body_str,
        })
    }
}

#[pymodule]
fn oxhttp(m: &Bound<'_, PyModule>) -> PyResult<()> {
    routing::routing(m)?;
    m.add_class::<HttpServer>()?;
    Ok(())
}
