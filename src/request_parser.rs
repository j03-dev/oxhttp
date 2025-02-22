use crate::request::Request;

use std::collections::HashMap;

pub struct RequestParser;

impl RequestParser {
    pub fn parse(request_str: &str) -> Option<Request> {
        let mut lines = request_str.lines();

        let request_line = lines.next()?;
        let parts: Vec<&str> = request_line.split_whitespace().collect();
        if parts.len() < 3 {
            return None;
        }

        let method = parts[0].to_string();
        let path = parts[1].to_string();

        let mut headers = HashMap::new();
        let mut body = String::new();
        let mut reading_headers = true;

        for line in lines {
            if reading_headers {
                if line.is_empty() {
                    reading_headers = false;
                    continue;
                }
                let header_parts: Vec<&str> = line.split(": ").collect();
                if header_parts.len() == 2 {
                    headers.insert(
                        header_parts[0].trim().to_string(),
                        header_parts[1].trim().to_string(),
                    );
                }
            } else {
                body.push_str(line);
                body.push('\n');
            }
        }

        if body.ends_with('\n') {
            body.pop();
        }

        Some(Request::new(method, path, headers, body))
    }
}
