use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{SocketAddr, TcpListener},
    sync::Arc,
};

use pyo3::{prelude::*, types::{PyDict, PyTuple}};
use regex::{Captures, Regex};

#[pyclass]
struct Route {
    method: String,
    path_pattern: String,
    regex: Regex,
    param_names: Vec<String>,
    handler: Py<PyAny>,
}

impl Route {
    fn new(method: String, path_pattern: String, handler: Py<PyAny>) -> Self {
        let (regex, param_names) = Self::compile_route_pattern(&path_pattern);
        Self {
            method,
            path_pattern,
            regex,
            param_names,
            handler,
        }
    }

    fn compile_route_pattern(pattern: &str) -> (Regex, Vec<String>) {
        let re = Regex::new(r"<([^>]+)>").unwrap();
        let mut param_names = Vec::new();

        let regex_pattern = re
            .replace_all(pattern, |caps: &Captures| {
                let param = &caps[1];
                param_names.push(param.to_string());
                format!(r"(?P<{}>[^/]+)", param)
            })
            .to_string();

        (
            Regex::new(&format!("^{}$", regex_pattern)).unwrap(),
            param_names,
        )
    }

    fn match_path(&self, path: &str) -> Option<HashMap<String, String>> {
        self.regex.captures(path).map(|caps| {
            self.param_names
                .iter()
                .filter_map(|name| {
                    caps.name(name)
                        .map(|m| (name.clone(), m.as_str().to_string()))
                })
                .collect()
        })
    }
}

#[pyclass]
struct HttpServer {
    addr: SocketAddr,
    routes: Vec<Arc<Route>>,
}

#[pymethods]
impl HttpServer {
    #[new]
    fn new(addr: (String, u16)) -> PyResult<Self> {
        let (ip, port) = addr;
        Ok(Self {
            addr: SocketAddr::new(ip.parse().unwrap(), port),
            routes: Vec::new(),
        })
    }

    fn route(&mut self, route: PyRef<'_, Route>) -> PyResult<()> {
        self.routes.push(Arc::new(Route::new(
            route.method.clone(),
            route.path_pattern.clone(),
            route.handler.clone_ref(route.py()),
        )));
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

            let mut response = None;
            for route in &self.routes {
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
                            Ok(resp) => response = Some(resp),
                            Err(e) => response = Some(format!(
                                "HTTP/1.1 500 Internal Server Error\r\nContent-Length: {}\r\n\r\n{}",
                                e.to_string().len(),
                                e.to_string()
                            )),
                        }
                        break;
                    }
                }
            }

            let response = response.unwrap_or_else(||
                "HTTP/1.1 404 Not Found\r\nContent-Length: 9\r\n\r\nNot Found".to_string()
            );

            socket.write_all(response.as_bytes())?;
            socket.flush()?;
        }
    }
}

impl HttpServer {
    fn process_response(&self, py: Python<'_>, handler: &Py<PyAny>, args: &Bound<'_, PyTuple>) -> PyResult<String> {
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
            let json_str = json_mod.call_method("dumps", (dict,), None)?.extract::<String>()?;
            ("application/json", json_str)
        } else {
            let repr = body.bind(py).repr()?.extract::<String>()?;
            ("text/plain", repr)
        };

        Ok(format!(
            "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n{}",
            status,
            status_reason(status),
            content_type,
            body_str.len(),
            body_str
        ))
    }
}

#[pyfunction]
fn get(path: String, handler: Py<PyAny>) -> Route {
    Route::new("GET".to_string(), path, handler)
}

#[pyfunction]
fn post(path: String, handler: Py<PyAny>) -> Route {
    Route::new("POST".to_string(), path, handler)
}

fn status_reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "Unknown",
    }
}

#[pymodule]
fn oxhttp(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<HttpServer>()?;
    m.add_function(wrap_pyfunction!(get, m)?)?;
    m.add_function(wrap_pyfunction!(post, m)?)?;
    Ok(())
}