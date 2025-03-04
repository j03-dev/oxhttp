use std::collections::HashMap;

use pyo3::prelude::*;

use crate::{into_response::IntoResponse, status::Status};

#[derive(Clone)]
#[pyclass]
pub struct Response {
    pub status: Status,
    pub body: String,
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
        py: Python,
    ) -> Self {
        Self {
            status: status.clone(),
            body: {
                match body.extract(py) {
                    Ok(body) => body,
                    _ => body.to_string(),
                }
            },
            headers: HashMap::from([("Content-Type".to_string(), content_type)]),
        }
    }

    pub fn header(&mut self, key: String, value: String) {
        self.headers.insert(key, value);
    }
}

impl IntoResponse for Response {
    fn into_response(&self) -> Response {
        self.clone()
    }
}

impl Response {
    pub fn body(mut self, body: String) -> Self {
        self.body = body;
        self
    }
}
