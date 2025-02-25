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

                for header in req.headers.iter() {
                    let name = header.name.to_string();
                    let value = String::from_utf8_lossy(header.value).to_string();
                    header_map.insert(name, value);
                }

                let mut request = Request::new(method, url, header_map);

                if let Some(body_start) = request_str[..header_size].rfind("\r\n\r\n") {
                    let body_start = body_start + 4;
                    if body_start < request_str.len() {
                        let body = Self::extract_body(request_str, &request.headers, body_start)
                            .map_err(|_| httparse::Error::Status)?;
                        if let Some(body) = body {
                            if !body.trim().is_empty() {
                                request.set_body(body);
                            }
                        }
                    }
                }

                Ok(request)
            }
            httparse::Status::Partial => Err(httparse::Error::Status),
        }
    }

    fn extract_body(
        request_str: &str,
        headers: &HashMap<String, String>,
        body_start: usize,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        if let Some(transfer_encoding) = headers.get("Transfer-Encoding") {
            if transfer_encoding.eq_ignore_ascii_case("chunked") {
                return Ok(Some(Self::handle_chunked_body(&request_str[body_start..])?));
            }
        }

        if let Some(content_length) = headers.get("Content-Length") {
            let length: usize = content_length.parse()?;
            if length > 0 && body_start + length <= request_str.len() {
                return Ok(Some(
                    request_str[body_start..body_start + length].to_string(),
                ));
            }
        }

        Ok(None)
    }

    fn handle_chunked_body(body_str: &str) -> Result<String, Box<dyn std::error::Error>> {
        let mut result = String::new();
        let mut remaining = body_str;

        while !remaining.is_empty() {
            if let Some(pos) = remaining.find("\r\n") {
                let size = usize::from_str_radix(&remaining[..pos], 16)?;
                if size == 0 {
                    break;
                }

                remaining = &remaining[pos + 2..];
                if remaining.len() < size {
                    break;
                }

                result.push_str(&remaining[..size]);
                remaining = &remaining[size + 2..];
            } else {
                break;
            }
        }

        Ok(result)
    }
}
