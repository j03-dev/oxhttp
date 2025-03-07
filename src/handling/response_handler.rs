use std::sync::Arc;

use pyo3::{
    types::{PyAnyMethods, PyDict},
    Bound, Py, PyAny, PyResult, Python,
};
use tokio::sync::mpsc::Receiver;

use crate::{
    into_response::{convert_to_response, IntoResponse},
    middleware::MiddlewareChain,
    request::Request,
    response::Response,
    routing::{Route, Router},
    status::Status,
    MatchitRoute, ProcessRequest,
};

pub async fn handle_response(shutdown_rx: &mut Receiver<()>, rx: &mut Receiver<ProcessRequest>) {
    loop {
        tokio::select! {
            Some(process_request) = rx.recv() => {
                let response = match process_response(
                    &process_request.router,
                    process_request.route,
                    &process_request.request,
                    process_request.app_data,
                ) {
                    Ok(response) => response,
                    Err(e) => Status::INTERNAL_SERVER_ERROR
                        .into_response()
                        .unwrap()
                        .body(e.to_string()),
                };

                let final_response = if let Some(cors) = process_request.cors {
                    cors.apply_to_response(response).unwrap()
                } else {
                    response
                };

                _ = process_request.response_sender.send(final_response).await;
            }
            _ = shutdown_rx.recv() => {break}
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
        setup_body(route, kwargs, request, py)?;

        let result = if !router.middlewares.is_empty() {
            let chain = MiddlewareChain::new(router.middlewares.clone());
            chain.execute(py, request, &route.handler.clone(), kwargs.clone())?
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
    if let (Some(ref app_data), true) = (app_data, route.args.contains(&"app_data".to_string())) {
        kwargs.set_item("app_data", app_data.clone_ref(py))?;
    }
    Ok(())
}

fn setup_body(
    route: &Route,
    kwargs: &Bound<'_, PyDict>,
    request: &Request,
    py: Python<'_>,
) -> PyResult<()> {
    if let Some(body_name) = &route.data {
        match route.content_type.as_str() {
            "application/json" => kwargs.set_item(body_name, request.json(py)?)?,
            _ => kwargs.set_item(body_name, request.body.clone())?,
        }
    }
    Ok(())
}
