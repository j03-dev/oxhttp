use std::{collections::HashMap, mem::transmute, sync::Arc};

use pyo3::{exceptions::PyException, ffi::c_str, prelude::*, pyclass, types::PyDict, Py, PyAny};
use std::ffi::CStr;

use crate::middleware::Middleware;

#[derive(Clone, Debug)]
#[pyclass]
pub struct Route {
    pub method: String,
    pub path: String,
    pub handler: Arc<Py<PyAny>>,
    pub args: Arc<HashMap<String, Option<String>>>,
}

#[pymethods]
impl Route {
    fn __repr__(&self) -> String {
        format!("{:#?}", self)
    }
}

impl Route {
    pub fn new(
        method: String,
        path: String,
        handler: Arc<Py<PyAny>>,
        py: Python<'_>,
    ) -> PyResult<Self> {
        let inspect = PyModule::import(py, "inspect")?;
        let sig = inspect.call_method("signature", (handler.clone_ref(py),), None)?;
        let parameters = sig.getattr("parameters")?;
        let values = parameters.call_method("values", (), None)?.try_iter()?;

        let extract_fields_code = include_str!("python/extract_fields.py");
        let extr = unsafe { CStr::from_ptr(extract_fields_code.as_ptr().cast()) };
        let globals = PyDict::new(py);
        py.run(extr, Some(&globals), None)?;
        let extract_fields = globals.get_item("extract_fields")?;

        let mut args = HashMap::new();

        for param in values {
            let param = param?.into_pyobject(py)?;
            let name: String = param.getattr("name")?.extract()?;
            let mut type_info = None;
            if let Ok(annotation) = param.getattr("annotation") {
                type_info = Some(
                    extract_fields
                        .as_ref()
                        .unwrap()
                        .call((annotation,), None)?
                        .to_string(),
                );
            }
            args.insert(name, type_info);
        }

        Ok(Route {
            method,
            path,
            handler,
            args: Arc::new(args),
        })
    }
}

macro_rules! methods {
    ($($method:ident),*) => {
        $(
            #[pyfunction]
            pub fn $method(path: String, handler: Py<PyAny>, py: Python<'_>) -> PyResult<Route> {
                Route::new(stringify!($method).to_string().to_uppercase(), path, Arc::new(handler), py)
            }
        )*
    }
}

methods!(get, post, put, patch, delete);

#[derive(Default, Clone, Debug)]
#[pyclass]
pub struct Router {
    pub routes: HashMap<String, matchit::Router<Route>>,
    pub middlewares: Vec<Middleware>,
}

#[pymethods]
impl Router {
    #[new]
    pub fn new() -> Self {
        Router::default()
    }

    fn middleware(&mut self, middleware: Py<PyAny>) {
        let middleware = Middleware::new(middleware);
        self.middlewares.push(middleware);
    }

    fn route(&mut self, route: PyRef<Route>) -> PyResult<()> {
        let method_router = self.routes.entry(route.method.clone()).or_default();
        method_router
            .insert(&route.path, route.clone())
            .map_err(|err| PyException::new_err(err.to_string()))?;
        Ok(())
    }
}

impl Router {
    pub fn find<'l>(&self, method: &str, uri: &str) -> Option<matchit::Match<'l, 'l, &'l Route>> {
        let path = uri.split('?').next().unwrap_or(uri);
        if let Some(router) = self.routes.get(method) {
            if let Ok(route) = router.at(path) {
                let route: matchit::Match<'l, 'l, &Route> = unsafe { transmute(route) };
                return Some(route);
            }
        }
        None
    }
}

#[pyfunction]
pub fn static_files(directory: String, path: String, py: Python<'_>) -> PyResult<Route> {
    let pathlib = py.import("pathlib")?;
    let oxhttp = py.import("oxhttp")?;

    let globals = &PyDict::new(py);
    globals.set_item("Path", pathlib.getattr("Path")?)?;
    globals.set_item("directory", directory)?;
    globals.set_item("Status", oxhttp.getattr("Status")?)?;
    globals.set_item("Response", oxhttp.getattr("Response")?)?;

    let handler = py.eval(
        c_str!(
            r#"lambda path: \
                Response(
                    Status.OK,
                    open(Path(directory) / path, 'rb')\
                        .read()\
                        .decode('utf-8'),
                    "text/plain",
                )\
                if (Path(directory) / path).exists()\
                else Status.NOT_FOUND"#
        ),
        Some(globals),
        None,
    )?;

    get(format!("/{path}/{{*path}}"), handler.into(), py)
}
