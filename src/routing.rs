use std::{collections::HashMap, mem::transmute, sync::Arc};

use pyo3::{exceptions::PyException, ffi::c_str, prelude::*, pyclass, types::PyDict, Py, PyAny};

#[derive(Clone, Debug)]
#[pyclass]
pub struct Route {
    pub method: String,
    pub path: String,
    pub handler: Arc<Py<PyAny>>,
    pub args: Arc<Vec<String>>,
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

        let mut args: Vec<String> = Vec::new();

        for param in values {
            let param = param?.into_pyobject(py)?;
            let name = param.getattr("name")?.extract()?;
            args.push(name);
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
    pub middleware: Option<Arc<Py<PyAny>>>,
}

#[pymethods]
impl Router {
    #[new]
    pub fn new() -> Self {
        Router::default()
    }

    fn middleware(&mut self, middleware: Py<PyAny>) {
        self.middleware = Some(Arc::new(middleware));
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
    pub fn find<'l>(&self, method: &str, url: &str) -> Option<matchit::Match<'l, 'l, &'l Route>> {
        if let Some(router) = self.routes.get(method) {
            if let Ok(route) = router.at(url) {
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

    get(format!("/{path}/{{*path}}"), handler.into(), py)
}
