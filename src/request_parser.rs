use crate::Request;
use std::collections::HashMap;

pub struct RequestParser;

impl RequestParser {
    pub fn parse(request_str: &str) -> Result<Request, httparse::Error> {
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);

        match req.parse(request_str.as_bytes())? {
            httparse::Status::Complete(header_size) => {
                let method = req.method.unwrap_or("GET").to_string();
                let url = req.path.unwrap_or("/").to_string();

                let mut header_map = HashMap::new();
                let mut content_type = String::from("text/plain");
                let mut content_length = 0;

                for header in req.headers.iter() {
                    let name = header.name.to_string();
                    let value = String::from_utf8_lossy(header.value).to_string();

                    match name.as_str() {
                        "Content-Type" => content_type = value.clone(),
                        "Content-Length" => {
                            content_length = value.parse().unwrap_or(0);
                        }
                        _ => {}
                    }

                    header_map.insert(name, value);
                }

                let mut request =
                    Request::new(method, url, content_type, content_length, header_map);

                if content_length > 0 {
                    let body = &request_str[header_size..];
                    request.set_body(body.to_string());
                }
                Ok(request)
            }
            httparse::Status::Partial => Err(httparse::Error::Status),
        }
    }
}
