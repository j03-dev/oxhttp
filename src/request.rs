use std::{collections::HashMap, sync::Arc};

use pyo3::prelude::*;

#[derive(Clone)]
#[pyclass]
pub struct Request {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

#[pymethods]
impl Request {
    #[new]
    pub fn new(method: String, url: String, headers: Vec<(String, String)>, body: String) -> Self {
        Self {
            method,
            url,
            headers,
            body,
        }
    }
}

#[derive(Clone)]
#[pyclass]
pub struct Context {
    pub request: Request,
    pub variables: HashMap<String, Arc<PyObject>>,
}

#[pymethods]
impl Context {
    #[new]
    pub fn new(request: Request) -> Self {
        Self {
            request,
            variables: HashMap::new(),
        }
    }

    fn set_variable(&mut self, key: String, value: PyObject) {
        self.variables.insert(key, Arc::new(value));
    }

    fn get_variable(&self, key: &str, py: Python<'_>) -> Py<PyAny> {
        self.variables.get(key).unwrap().clone_ref(py).into_any()
    }
}
