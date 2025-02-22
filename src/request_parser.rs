use crate::request::Request;

use std::collections::HashMap;

pub struct RequestParser;

impl RequestParser {
    pub fn parse(request_str: &str) -> Option<Request> {
        let parts: Vec<&str> = request_str.split_whitespace().collect();
        if parts.len() < 3 {
            return None;
        }

        let method = parts[0].to_string();
        let path = parts[1].to_string();

        let mut headers = HashMap::new();
        let mut body = String::new();
        let mut is_body = false;

        for line in request_str.lines().skip(1) {
            if is_body {
                body.push_str(line);
                body.push('\n');
            } else if line.is_empty() {
                is_body = true;
            }
            let header_parts: Vec<&str> = line.split(": ").collect();
            if header_parts.len() == 2 {
                headers.insert(header_parts[0].to_string(), header_parts[1].to_string());
            }
        }

        let request = Request::new(method, path, headers, body);
        Some(request)
    }
}
