use crate::{status::Status, Response};
use pyo3::{types::PyDict, Py};

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

#[macro_export]
macro_rules! to_response {
    ($rslt:expr, $py:expr) => {{
        if let Ok(response) = $rslt.extract::<PyRef<'_, Response>>($py) {
            return Ok(response.clone());
        }

        if let Ok(status) = $rslt.extract::<PyRef<'_, Status>>($py) {
            return Ok(status.into_response());
        }

        if let Ok((body, status_code)) = $rslt.extract::<(String, Status)>($py) {
            return Ok((body, status_code).into_response());
        }

        if let Ok((body, status_code)) = $rslt.extract::<(Py<PyDict>, Status)>($py) {
            return Ok((body, status_code).into_response());
        }

        if let Ok(body) = $rslt.extract::<Py<PyDict>>($py) {
            return Ok(body.into_response());
        }

        if let Ok(body) = $rslt.extract::<String>($py) {
            return Ok(body.into_response());
        }

        if let Ok(body) = $rslt.extract::<i32>($py) {
            return Ok(body.into_response());
        }

        return Err(pyo3::exceptions::PyException::new_err(
            "Failed to convert this type to response",
        ));
    }};
}
