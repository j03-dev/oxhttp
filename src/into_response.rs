use pyo3::{types::PyDict, Py};

use crate::{status::Status, Response};

pub trait IntoResponse {
    fn into_response(&self) -> Response;
}

impl IntoResponse for String {
    fn into_response(&self) -> Response {
        Response {
            status: Status::OK(),
            content_type: "text/plain".to_string(),
            body: self.clone(),
        }
    }
}

impl IntoResponse for Py<PyDict> {
    fn into_response(&self) -> Response {
        Response {
            status: Status::OK(),
            content_type: "text/plain".to_string(),
            body: self.to_string(),
        }
    }
}

impl IntoResponse for (String, u16) {
    fn into_response(&self) -> Response {
        Response {
            status: Status(self.1),
            content_type: "text/plain".to_string(),
            body: self.0.clone(),
        }
    }
}

impl IntoResponse for (Py<PyDict>, u16) {
    fn into_response(&self) -> Response {
        Response {
            status: Status(self.1),
            content_type: "json/application".to_string(),
            body: self.0.to_string(),
        }
    }
}

impl IntoResponse for (String, Status) {
    fn into_response(&self) -> Response {
        Response {
            status: self.1.clone(),
            content_type: "text/plain".to_string(),
            body: self.0.clone(),
        }
    }
}

impl IntoResponse for (Py<PyDict>, Status) {
    fn into_response(&self) -> Response {
        Response {
            status: self.1.clone(),
            content_type: "json/application".to_string(),
            body: self.0.to_string(),
        }
    }
}
