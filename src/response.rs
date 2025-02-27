use pyo3::prelude::*;

use crate::{into_response::IntoResponse, status::Status};

#[derive(Clone)]
#[pyclass]
pub struct Response {
    pub status: Status,
    pub content_type: String,
    pub body: String,
}

#[pymethods]
impl Response {
    #[new]
    #[pyo3(signature=(status, body, content_type="json/application".to_string()))]
    pub fn new(
        status: PyRef<'_, Status>,
        body: PyObject,
        content_type: String,
        py: Python,
    ) -> Self {
        Self {
            status: status.clone(),
            content_type,
            body: {
                match body.extract(py) {
                    Ok(body) => body,
                    _ => body.to_string(),
                }
            },
        }
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
