use std::collections::HashMap;

use pyo3::{prelude::*, types::PyDict};

#[derive(Clone)]
#[pyclass]
pub struct Request {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

#[pymethods]
impl Request {
    #[new]
    pub fn new(
        method: String,
        url: String,
        headers: HashMap<String, String>,
        body: String,
    ) -> Self {
        Self {
            method,
            url,
            headers,
            body,
        }
    }

    pub fn headers(&self) -> HashMap<String, String> {
        self.headers.clone()
    }

    pub fn json(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let json_mod = PyModule::import(py, "json")?;
        json_mod
            .call_method("loads", (self.body.clone(),), None)?
            .extract()
    }
}
