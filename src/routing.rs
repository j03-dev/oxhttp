use pyo3::prelude::*;

use regex::{Captures, Regex};
use std::collections::HashMap;
use std::sync::Arc;

type Middleware = Py<PyAny>;

#[derive(Clone)]
#[pyclass]
pub struct Router {
    pub(crate) routes: Vec<Arc<Route>>,
    pub(crate) middlewares: Vec<Arc<Middleware>>,
}

#[pymethods]
impl Router {
    #[new]
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            middlewares: Vec::new(),
        }
    }

    fn middleware(&mut self, middleware: Middleware) {
        self.middlewares.push(Arc::new(middleware));
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

macro_rules! method {
    ($($func:ident),+) => {
        $(
            #[pyfunction]
            pub fn $func(path: String, handler: Py<PyAny>) -> Route {
                let method_name = stringify!($func).to_uppercase();
                Route::new(method_name, path, handler)
            }
        )+
    };
}

method!(get, post, delete, patch, put);
