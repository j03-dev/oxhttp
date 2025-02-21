use pyo3::ffi::c_str;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use regex::{Captures, Regex};
use std::collections::HashMap;
use std::sync::Arc;

type Middleware = Py<PyAny>;

#[derive(Clone, Default)]
#[pyclass]
pub struct Router {
    pub(crate) routes: Vec<Arc<Route>>,
    pub(crate) middleware: Option<Arc<Middleware>>,
}

#[pymethods]
impl Router {
    #[new]
    pub fn new() -> Self {
        Self::default()
    }

    fn middleware(&mut self, middleware: Middleware) {
        self.middleware = Some(Arc::new(middleware));
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
                let param_name = if param.contains(":") {
                    param.split(":").next().unwrap()
                } else {
                    param
                };
                param_names.push(param_name.to_string());
                format!(r"(?P<{}>.+)", param_name)
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

macro_rules! methods {
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

methods!(get, post, delete, patch, put);

#[pyfunction]
pub fn static_files(directory: String, path: String, py: Python<'_>) -> PyResult<Route> {
    let pathlib = py.import("pathlib")?;
    let oxhttp = py.import("oxhttp")?;

    let locals = &PyDict::new(py);
    locals.set_item("Path", pathlib.getattr("Path")?)?;
    locals.set_item("directory", directory)?;
    locals.set_item("Response", oxhttp.getattr("Response")?)?;
    locals.set_item("Status", oxhttp.getattr("Status")?)?;

    let handler = py.eval(
        c_str!(
            r#"lambda path: \
                Response(
                    Status.OK(),
                    open(Path(directory) / path, 'rb')\
                        .read()\
                        .decode('utf-8')\
                )\
                if (Path(directory) / path).exists()\
                else Status.NOT_FOUND()"#
        ),
        None,
        Some(locals),
    )?;

    get(format!("/{path}/<path:path>"), handler.into(), py)
}
