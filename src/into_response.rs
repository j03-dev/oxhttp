use std::collections::HashMap;

use crate::{status::Status, Response};
use pyo3::{prelude::*, types::PyAny, Py};

pub trait IntoResponse {
    #[allow(clippy::wrong_self_convention)]
    fn into_response(&self) -> PyResult<Response>;
}

impl IntoResponse for String {
    fn into_response(&self) -> PyResult<Response> {
        Ok(Response {
            status: Status::OK,
            headers: HashMap::from([("Content-Type".to_string(), "text/plain".to_string())]),
            body: self.clone(),
        })
    }
}

impl IntoResponse for PyObject {
    fn into_response(&self) -> PyResult<Response> {
        Ok(Response {
            status: Status::OK,
            headers: HashMap::from([("Content-Type".to_string(), "application/json".to_string())]),
            body: crate::json::dumps(self)?,
        })
    }
}

impl IntoResponse for (String, Status) {
    fn into_response(&self) -> PyResult<Response> {
        Ok(Response {
            status: self.1.clone(),
            headers: HashMap::from([("Content-Type".to_string(), "text/plain".to_string())]),
            body: self.0.clone(),
        })
    }
}

impl IntoResponse for (PyObject, Status) {
    fn into_response(&self) -> PyResult<Response> {
        Ok(Response {
            status: self.1.clone(),
            headers: HashMap::from([("Content-Type".to_string(), "application/json".to_string())]),
            body: crate::json::dumps(&self.0)?,
        })
    }
}

macro_rules! to_response {
    ($rslt:expr, $py:expr, $($type:ty),*) => {{
        $(
            if let Ok(value) = $rslt.extract::<$type>($py) {
                return value.into_response();
            }
        )*

        return Err(pyo3::exceptions::PyException::new_err(
            "Failed to convert this type to response",
        ));
    }};
}

pub fn convert_to_response(result: Py<PyAny>, py: Python<'_>) -> PyResult<Response> {
    to_response!(
        result,
        py,
        PyRef<'_, Response>,
        PyRef<'_, Status>,
        (String, Status),
        (PyObject, Status),
        String,
        PyObject
    )
}
