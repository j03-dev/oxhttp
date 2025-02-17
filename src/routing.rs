use pyo3::prelude::*;

use regex::{Captures, Regex};
use std::collections::HashMap;
use std::sync::Arc;

#[pyclass]
pub struct Router {
    pub(crate) routes: Vec<Arc<Route>>,
}

#[pymethods]
impl Router {
    #[new]
    pub fn new() -> Self {
        Self { routes: Vec::new() }
    }

    fn route(&mut self, route: PyRef<'_, Route>) -> PyResult<()> {
        self.routes.push(Arc::new(Route::new(
            route.method.clone(),
            route.path_pattern.clone(),
            route.handler.clone_ref(route.py()),
        )));
        Ok(())
    }
}

#[pyclass]
pub(crate) struct Route {
    pub(crate) method: String,
    pub(crate) path_pattern: String,
    pub(crate) regex: Regex,
    pub(crate) param_names: Vec<String>,
    pub(crate) handler: Py<PyAny>,
}

impl Route {
    pub(crate) fn new(method: String, path_pattern: String, handler: Py<PyAny>) -> Self {
        let (regex, param_names) = Self::compile_route_pattern(&path_pattern);
        Self {
            method,
            path_pattern,
            regex,
            param_names,
            handler,
        }
    }

    fn compile_route_pattern(pattern: &str) -> (Regex, Vec<String>) {
        let re = Regex::new(r"<([^>]+)>").unwrap();
        let mut param_names = Vec::new();

        let regex_pattern = re
            .replace_all(pattern, |caps: &Captures| {
                let param = &caps[1];
                param_names.push(param.to_string());
                format!(r"(?P<{}>[^/]+)", param)
            })
            .to_string();

        (
            Regex::new(&format!("^{}$", regex_pattern)).unwrap(),
            param_names,
        )
    }

    pub(crate) fn match_path(&self, path: &str) -> Option<HashMap<String, String>> {
        self.regex.captures(path).map(|caps| {
            self.param_names
                .iter()
                .filter_map(|name| {
                    caps.name(name)
                        .map(|m| (name.clone(), m.as_str().to_string()))
                })
                .collect()
        })
    }
}

#[pyfunction]
fn get(path: String, handler: Py<PyAny>) -> Route {
    Route::new("GET".to_string(), path, handler)
}

#[pyfunction]
fn post(path: String, handler: Py<PyAny>) -> Route {
    Route::new("POST".to_string(), path, handler)
}

#[pyfunction]
fn delete(path: String, handler: Py<PyAny>) -> Route {
    Route::new("DELETE".to_string(), path, handler)
}

#[pyfunction]
fn patch(path: String, handler: Py<PyAny>) -> Route {
    Route::new("PATCH".to_string(), path, handler)
}

#[pyfunction]
fn put(path: String, handler: Py<PyAny>) -> Route {
    Route::new("PUT".to_string(), path, handler)
}

pub fn routing(parent_module: &Bound<'_, PyModule>) -> PyResult<()> {
    let routing_module = PyModule::new(parent_module.py(), "routing")?;

    routing_module.add_class::<Router>()?;
    routing_module.add_function(wrap_pyfunction!(get, &routing_module)?)?;
    routing_module.add_function(wrap_pyfunction!(post, &routing_module)?)?;
    routing_module.add_function(wrap_pyfunction!(delete, &routing_module)?)?;
    routing_module.add_function(wrap_pyfunction!(patch, &routing_module)?)?;
    routing_module.add_function(wrap_pyfunction!(put, &routing_module)?)?;

    parent_module.add_submodule(&routing_module)
}
