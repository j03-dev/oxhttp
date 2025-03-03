use std::{collections::HashMap, mem::transmute, sync::Arc};

use pyo3::{exceptions::PyException, ffi::c_str, prelude::*, pyclass, types::PyDict, Py, PyAny};

use crate::middleware::Middleware;

#[derive(Clone, Debug)]
#[pyclass]
pub struct Route {
    pub method: String,
    pub path: String,
    pub handler: Arc<Py<PyAny>>,
    pub args: Arc<Vec<String>>,
    pub content_type: String,
    pub data: Option<String>,
}

#[pymethods]
impl Route {
    #[new]
    #[pyo3(signature=(path, method=None, content_type=None, data=None))]
    pub fn new(
        path: String,
        method: Option<String>,
        content_type: Option<String>,
        data: Option<String>,
    ) -> Self {
        Route {
            method: method.unwrap_or_else(|| "GET".to_string()),
            path,
            handler: Arc::new(Python::with_gil(|py| py.None())),
            args: Arc::new(Vec::new()),
            content_type: content_type.unwrap_or_else(|| "application/json".to_string()),
            data,
        }
    }

    fn __call__(&self, handler: Py<PyAny>, py: Python<'_>) -> PyResult<Self> {
        let inspect = PyModule::import(py, "inspect")?;
        let sig = inspect.call_method("signature", (handler.clone_ref(py),), None)?;
        let parameters = sig.getattr("parameters")?;
        let values = parameters.call_method("values", (), None)?.try_iter()?;

        let mut args: Vec<String> = Vec::new();

        for param in values {
            let param = param?.into_pyobject(py)?;
            let name = param.getattr("name")?.extract()?;
            args.push(name);
        }

        if let Some(data) = self.data.clone() {
            if !args.contains(&data) {
                let message = format!("Missing argument '{data}'");
                return Err(PyException::new_err(message));
            }
        }

        Ok(Self {
            handler: Arc::new(handler),
            args: Arc::new(args),
            ..self.clone()
        })
    }

    fn __repr__(&self) -> String {
        format!("{:#?}", self.clone())
    }
}

macro_rules! method_decorator {
    ($($method:ident),*) => {
        $(
            #[pyfunction]
            #[pyo3(signature = (path, *, content_type=None, data=None))]
            pub fn $method(path: String, content_type: Option<String>, data: Option<String>)-> Route{
                Route::new(
                    path,
                    Some(stringify!($method).to_string().to_uppercase()),
                    content_type,
                    data,
                )
            }
        )+
    };
}

method_decorator!(get, post, put, patch, delete);

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

    fn routes(&mut self, routes: Vec<PyRef<Route>>) -> PyResult<()> {
        for route in routes {
            self.route(route)?;
        }
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

    let route = Route::new(
        format!("/{path}/{{*path}}"),
        Some("GET".to_string()),
        Some("text/plain".to_string()),
        None,
    );

    route.__call__(handler.into(), py)
}
