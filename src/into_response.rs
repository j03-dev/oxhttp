use crate::{Response, status::Status};
use pyo3::{
    Py,
    prelude::*,
    types::{PyAny, PyDict},
};

pub trait IntoResponse {
    #[allow(clippy::wrong_self_convention)]
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
            content_type: "application/json".to_string(),
            body: self.to_string(),
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
            content_type: "application/json".to_string(),
            body: self.0.to_string(),
        }
    }
}

impl IntoResponse for i32 {
    fn into_response(&self) -> Response {
        Response {
            status: Status::OK(),
            content_type: "application/json".to_string(),
            body: self.to_string(),
        }
    }
}

macro_rules! to_response {
    ($rslt:expr, $py:expr, $($type:ty),*) => {{
        $(
            if let Ok(value) = $rslt.extract::<$type>($py) {
                return Ok(value.into_response());
            }
        )*

        return Err(pyo3::exceptions::PyException::new_err(
            "Failed to convert this type to response",
        ));
    }};
}

pub fn convert(result: Py<PyAny>, py: Python<'_>) -> PyResult<Response> {
    to_response!(
        result,
        py,
        PyRef<'_, Response>,
        PyRef<'_, Status>,
        (String, Status),
        (Py<PyDict>, Status),
        Py<PyDict>,
        String,
        i32
    )
}
