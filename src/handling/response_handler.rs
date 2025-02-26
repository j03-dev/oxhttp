use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use pyo3::{
    types::{PyAnyMethods, PyDict},
    Bound, Py, PyAny, PyResult, Python,
};
use tokio::sync::mpsc::Receiver;

use crate::{
    into_response::{convert_to_response, IntoResponse},
    request::Request,
    response::Response,
    routing::{Route, Router},
    status::Status,
    MatchitRoute, ProcessRequest,
};

pub async fn handle_response(running: Arc<AtomicBool>, rx: &mut Receiver<ProcessRequest>) {
    while running.load(Ordering::SeqCst) {
        if let Ok(process_request) = rx.try_recv() {
            let response = match process_response(
                &process_request.router,
                process_request.route,
                &process_request.request,
                process_request.app_data,
            ) {
                Ok(response) => response,
                Err(e) => Status::INTERNAL_SERVER_ERROR()
                    .into_response()
                    .body(e.to_string()),
            };

            _ = process_request.response_sender.send(response).await;
        }
    }
}

fn process_response(
    router: &Router,
    matchit_route: MatchitRoute,
    request: &Request,
    app_data: Option<Arc<Py<PyAny>>>,
) -> PyResult<Response> {
    Python::with_gil(|py| {
        let kwargs = &PyDict::new(py);
        let params = &matchit_route.params;
        let route = matchit_route.value;
        let app_data = app_data.clone();

        setup_params(kwargs, params)?;
        setup_app_data(app_data, route, kwargs, py)?;
        setup_body(route, kwargs, params, request, py)?;

        let result = if let Some(middleware) = &router.middleware {
            kwargs.set_item("request", request.clone())?;
            kwargs.set_item("next", route.handler.clone_ref(py))?;
            middleware.call(py, (), Some(kwargs))?
        } else {
            route.handler.call(py, (), Some(kwargs))?
        };

        convert_to_response(result, py)
    })
}

fn setup_params(kwargs: &Bound<'_, PyDict>, params: &matchit::Params<'_, '_>) -> PyResult<()> {
    for (key, value) in params.iter() {
        kwargs.set_item(key, value)?;
    }
    Ok(())
}
fn setup_app_data(
    app_data: Option<Arc<Py<PyAny>>>,
    route: &Route,
    kwargs: &Bound<'_, PyDict>,
    py: Python<'_>,
) -> PyResult<()> {
    if let (Some(app_data), true) = (
        app_data.as_ref(),
        route.args.contains(&"app_data".to_string()),
    ) {
        kwargs.set_item("app_data", app_data.clone_ref(py))?;
    }
    Ok(())
}

fn setup_body(
    route: &Route,
    kwargs: &Bound<'_, PyDict>,
    params: &matchit::Params<'_, '_>,
    request: &Request,
    py: Python<'_>,
) -> PyResult<()> {
    let mut body_param_name = None;

    for key in route.args.as_ref() {
        if key != "app_data"
            && params
                .iter()
                .filter(|(k, _)| *k == key)
                .collect::<Vec<_>>()
                .is_empty()
        {
            body_param_name = Some(key);
            break;
        }
    }

    if let (Some(ref body_name), Ok(ref body)) = (body_param_name, request.json(py)) {
        kwargs.set_item(body_name, body)?;
    }
    Ok(())
}
