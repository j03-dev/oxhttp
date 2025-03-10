use std::{collections::HashMap, sync::Arc};

use pyo3::{prelude::*, types::PyDict, IntoPyObjectExt};

use crate::to_py_exception;

#[pyclass]
pub struct Tera {
    engine: Arc<tera::Tera>,
}

#[pymethods]
impl Tera {
    #[new]
    fn new(dir: String) -> PyResult<Self> {
        Ok(Self {
            engine: Arc::new(to_py_exception(tera::Tera::new(&dir))?),
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
            let map: HashMap<String, serde_json::Value> =
                to_py_exception(serde_json::from_str(&serialize))?;
            for (key, value) in map {
                tera_context.insert(key, &value);
            }
        }

        to_py_exception(self.engine.render(&template_name, &tera_context))
    }
}
