use std::collections::HashMap;

use pyo3::{exceptions::PyException, prelude::*};

#[derive(Clone)]
#[pyclass]
pub struct Request {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

#[pymethods]
impl Request {
    #[new]
    pub fn new(
        method: String,
        url: String,
        headers: HashMap<String, String>,
        body: String,
    ) -> Self {
        Self {
            method,
            url,
            headers,
            body,
        }
    }

    pub fn headers(&self) -> HashMap<String, String> {
        self.headers.clone()
    }

    pub fn json(&self) -> PyResult<HashMap<String, String>> {
        let json_data = serde_json::from_str(&self.body)
            .map_err(|err| PyException::new_err(err.to_string()))?;
        Ok(json_data)
    }

    pub fn url(&self) -> String {
        self.url.clone()
    }

    pub fn method(&self) -> String {
        self.method.clone()
    }

    pub fn query(&self) -> PyResult<Option<HashMap<String, String>>> {
        let query_string = self.url.split('?').nth(1);
        if let Some(query) = query_string {
            let query_params = Self::parse_query_string(query.to_string());
            return Ok(Some(query_params));
        }
        Ok(None)
    }
}

impl Request {
    fn parse_query_string(query_string: String) -> HashMap<String, String> {
        query_string
            .split('&')
            .filter_map(|pair| {
                let mut parts = pair.split('=');
                let key = parts.next()?.to_string();
                let value = parts.next().map_or("".to_string(), |v| v.to_string());
                Some((key, value))
            })
            .collect()
    }
}
