use pyo3::{IntoPyObjectExt, prelude::*};

use crate::{convert, into_response::IntoResponse, status::Status};

use std::fmt;

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
    #[pyo3(signature=(status, body, content_type="json/application".to_string()))]
    pub(crate) fn new(
        status: PyRef<'_, Status>,
        body: PyObject,
        content_type: String,
        py: Python<'_>,
    ) -> PyResult<Self> {
        let rslt = Self {
            status: status.clone(),
            content_type,
            body: body.to_string(),
        }
        .into_py_any(py)?;
        convert(rslt, py)
    }
}

impl IntoResponse for Response {
    fn into_response(&self) -> Response {
        self.clone()
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
