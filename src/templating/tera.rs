use std::collections::HashMap;

use pyo3::{exceptions::PyException, prelude::*, types::PyDict, IntoPyObjectExt};

#[pyclass]
pub struct Tera {
    engine: tera::Tera,
}

#[pymethods]
impl Tera {
    #[new]
    fn new(dir: String) -> PyResult<Self> {
        Ok(Self {
            engine: tera::Tera::new(&dir).map_err(|err| PyException::new_err(err.to_string()))?,
        })
    }

    #[pyo3(signature=(template_name, context=None))]
    fn render(
        &mut self,
        template_name: String,
        context: Option<Bound<'_, PyDict>>,
        py: Python<'_>,
    ) -> PyResult<String> {
        let mut tera_context = tera::Context::new();

        if let Some(context) = context {
            let serialize = crate::json::dumps(&context.into_py_any(py)?)?;
            let map: HashMap<String, serde_json::Value> = serde_json::from_str(&serialize)
                .map_err(|err| PyException::new_err(err.to_string()))?;
            for (key, value) in map {
                tera_context.insert(key, &value);
            }
        }

        self.engine
            .render(&template_name, &tera_context)
            .map_err(|err| PyException::new_err(err.to_string()))
    }
}
