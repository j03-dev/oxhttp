use std::{mem::transmute, sync::Arc};

use http_body_util::{BodyExt, Full};
use hyper::{
    body::{Bytes, Incoming},
    Request as HyperRequest, Response as HyperResponse,
};
use pyo3::{Py, PyAny};
use tokio::sync::mpsc::{channel, Sender};

use crate::{
    into_response::IntoResponse, request::Request, response::Response, routing::Router,
    status::Status, MatchitRoute, ProcessRequest,
};

pub async fn handle_request(
    req: HyperRequest<Incoming>,
    request_sender: Sender<ProcessRequest>,
    routers: Vec<Arc<Router>>,
    app_data: Option<Arc<Py<PyAny>>>,
    channel_capacity: usize,
) -> Result<HyperResponse<Full<Bytes>>, hyper::http::Error> {
    let sender = request_sender.clone();
    let routers = routers.clone();

    let request = convert_hyper_request(req).await.unwrap();

    for router in &routers {
        if let Some(route) = router.find(&request.method, &request.url) {
            let (response_sender, mut respond_receive) = channel(channel_capacity);

            let route: MatchitRoute = unsafe { transmute(&route) };

            let process_request = ProcessRequest {
                request: request.clone(),
                router: router.clone(),
                route,
                response_sender,
                app_data,
            };

            if sender.send(process_request).await.is_ok() {
                if let Some(response) = respond_receive.recv().await {
                    return convert_to_hyper_response(response);
                }
            }
            break;
        }
    }

    convert_to_hyper_response(Status::FOUND().into_response())
}

async fn convert_hyper_request(
    req: HyperRequest<Incoming>,
) -> Result<Request, Box<dyn std::error::Error + Sync + Send>> {
    let method = req.method().to_string();
    let uri = req.uri().to_string();

    let mut headers = std::collections::HashMap::new();
    for (key, value) in req.headers() {
        headers.insert(
            key.to_string(),
            value.to_str().unwrap_or_default().to_string(),
        );
    }

    let mut request = Request::new(method, uri, headers);

    let body_bytes = req.collect().await?.to_bytes();
    let body = String::from_utf8_lossy(&body_bytes).to_string();
    if !body.is_empty() {
        request.set_body(body);
    }

    Ok(request)
}

fn convert_to_hyper_response(
    response: Response,
) -> Result<HyperResponse<Full<Bytes>>, hyper::http::Error> {
    HyperResponse::builder()
        .status(response.status.code())
        .header("Content-Type", response.content_type)
        .body(Full::new(Bytes::from(response.body)))
}
