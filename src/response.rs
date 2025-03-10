use std::collections::HashMap;

use hyper::body::Bytes;
use pyo3::{prelude::*, types::PyBytes};

use crate::{into_response::IntoResponse, status::Status};

#[derive(Clone)]
#[pyclass]
pub struct Response {
    pub status: Status,
    pub body: Bytes,
    pub headers: HashMap<String, String>,
}

#[pymethods]
impl Response {
    #[new]
    #[pyo3(signature=(status, body, content_type="application/json".to_string()))]
    pub fn new(
        status: PyRef<'_, Status>,
        body: PyObject,
        content_type: String,
        py: Python<'_>,
    ) -> PyResult<Self> {
        let body = if let Ok(bytes) = body.extract::<Py<PyBytes>>(py) {
            bytes.as_bytes(py).to_vec().into()
        } else if content_type == "application/json" {
            crate::json::dumps(&body)?.into()
        } else {
            body.to_string().into()
        };

        Ok(Self {
            status: status.clone(),
            body,
            headers: HashMap::from([("Content-Type".to_string(), content_type)]),
        })
    }

    pub fn header(&mut self, key: String, value: String) {
        self.headers.insert(key, value);
    }
}

impl IntoResponse for Response {
    fn into_response(&self) -> PyResult<Response> {
        Ok(self.clone())
    }
}

impl Response {
    pub fn body(mut self, body: String) -> Self {
        self.body = body.into();
        self
    }
}
