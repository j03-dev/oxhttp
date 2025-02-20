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
        self.routes.push(Arc::new(route.clone()));
        Ok(())
    }
}

#[derive(Clone)]
#[pyclass]
pub struct Route {
    pub method: String,
    pub regex: Regex,
    pub param_names: Vec<String>,
    pub handler: Arc<Py<PyAny>>,
    pub args: Vec<String>,
}

impl Route {
    pub fn new(
        method: String,
        path_pattern: String,
        handler: Py<PyAny>,
        py: Python<'_>,
    ) -> PyResult<Self> {
        let (regex, param_names) = Self::compile_route_pattern(&path_pattern);

        let inspect = PyModule::import(py, "inspect")?;
        let sig = inspect.call_method("signature", (handler.clone_ref(py),), None)?;
        let parameters = sig.getattr("parameters")?;
        let values = parameters.call_method("values", (), None)?.try_iter()?;

        let mut args: Vec<String> = Vec::new();

        for param in values {
            let param = param?.into_pyobject(py)?;
            let key = param.getattr("name")?.extract()?;
            args.push(key);
        }

        Ok(Self {
            method,
            regex,
            param_names,
            handler: Arc::new(handler),
            args,
        })
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

    pub fn match_path(&self, path: &str) -> Option<HashMap<String, String>> {
        let base_path = path.split('?').next()?;

        self.regex.captures(base_path).map(|caps| {
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
            pub fn $func(path: String, handler: Py<PyAny>, py: Python<'_>) -> PyResult<Route> {
                let method_name = stringify!($func).to_uppercase();
                Route::new(method_name, path, handler, py)
            }
        )+
    };
}

method!(get, post, delete, patch, put);
