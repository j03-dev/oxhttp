use pyo3::{prelude::*, types::PyDict};

use crate::status::Status;

use std::fmt;

pub trait IntoResponse {
    fn into_response(&self) -> Response;
}

#[derive(Clone)]
#[pyclass]
pub(crate) struct Response {
    pub(crate) status: Status,
    pub(crate) content_type: String,
    pub(crate) body: String,
}

#[pymethods]
impl Response {
    #[new]
    pub(crate) fn new(status: PyRef<'_, Status>, body: PyObject, py: Python<'_>) -> PyResult<Self> {
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

impl Response {
    pub(crate) fn body(mut self, body: String) -> Self {
        self.body = body;
        self
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
