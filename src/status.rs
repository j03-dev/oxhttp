use std::collections::HashMap;

use pyo3::prelude::*;

use crate::{into_response::IntoResponse, response::Response};

#[derive(Clone)]
#[pyclass]
pub struct Status(pub u16);

macro_rules! status_codes {
    ($(($num:expr, $kconst:ident);)+) => {
        #[pymethods]
        impl Status {
        $(
            #[classattr]
            pub const $kconst: Status = Status($num);
        )+

            #[getter]
            pub fn code(&self) -> u16 {
                self.0
            }
        }
    }
}

impl IntoResponse for Status {
    fn into_response(&self) -> PyResult<Response> {
        Ok(Response {
            status: self.clone(),
            headers: HashMap::from([("Content-Type".to_string(), "text/plain".to_string())]),
            body: String::new(),
        })
    }
}

status_codes! {
    (100, CONTINUE);
    (101, SWITCHING_PROTOCOLS);
    (102, PROCESSING);

    (200, OK);
    (201, CREATED);
    (202, ACCEPTED);
    (203, NON_AUTHORITATIVE_INFORMATION);
    (204, NO_CONTENT);
    (205, RESET_CONTENT);
    (206, PARTIAL_CONTENT);
    (207, MULTI_STATUS);
    (208, ALREADY_REPORTED);

    (226, IM_USED);

    (300, MULTIPLE_CHOICES);
    (301, MOVED_PERMANENTLY);
    (302, FOUND);
    (303, SEE_OTHER);
    (304, NOT_MODIFIED);
    (305, USE_PROXY);
    (307, TEMPORARY_REDIRECT);
    (308, PERMANENT_REDIRECT);

    (400, BAD_REQUEST);
    (401, UNAUTHORIZED);
    (402, PAYMENT_REQUIRED);
    (403, FORBIDDEN);
    (404, NOT_FOUND);
    (405, METHOD_NOT_ALLOWED);
    (406, NOT_ACCEPTABLE);
    (407, PROXY_AUTHENTICATION_REQUIRED);
    (408, REQUEST_TIMEOUT);
    (409, CONFLICT);
    (410, GONE);
    (411, LENGTH_REQUIRED);
    (412, PRECONDITION_FAILED);
    (413, PAYLOAD_TOO_LARGE);
    (414, URI_TOO_LONG);
    (415, UNSUPPORTED_MEDIA_TYPE);
    (416, RANGE_NOT_SATISFIABLE);
    (417, EXPECTATION_FAILED);
    (418, IM_A_TEAPOT);

    (421, MISDIRECTED_REQUEST);
    (422, UNPROCESSABLE_ENTITY);
    (423, LOCKED);
    (424, FAILED_DEPENDENCY);

    (426, UPGRADE_REQUIRED);

    (428, PRECONDITION_REQUIRED);
    (429, TOO_MANY_REQUESTS);

    (431, REQUEST_HEADER_FIELDS_TOO_LARGE);

    (451, UNAVAILABLE_FOR_LEGAL_REASONS);

    (500, INTERNAL_SERVER_ERROR);
    (501, NOT_IMPLEMENTED);
    (502, BAD_GATEWAY);
    (503, SERVICE_UNAVAILABLE);
    (504, GATEWAY_TIMEOUT);
    (505, HTTP_VERSION_NOT_SUPPORTED);
    (506, VARIANT_ALSO_NEGOTIATES);
    (507, INSUFFICIENT_STORAGE);
    (508, LOOP_DETECTED);

    (510, NOT_EXTENDED);
    (511, NETWORK_AUTHENTICATION_REQUIRED);
}
