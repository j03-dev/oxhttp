use std::{collections::HashMap, sync::Arc};

use minijinja::Environment;
use pyo3::{prelude::*, types::PyDict, IntoPyObjectExt};

use crate::to_py_exception;

#[derive(Debug)]
#[pyclass]
pub struct Jinja {
    engine: Arc<Environment<'static>>,
}

#[pymethods]
impl Jinja {
    #[new]
    fn new(dir: String) -> PyResult<Self> {
        let mut env = Environment::new();

        let paths = to_py_exception(glob::glob(&dir))?;

        for entry in paths {
            let path = to_py_exception(entry)?;
            if path.is_file() {
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "unknown.html".to_string());
                let content = to_py_exception(std::fs::read_to_string(&path))?;
                let name = Box::leak(name.into_boxed_str());
                let content = Box::leak(content.into_boxed_str());
                to_py_exception(env.add_template(name, content))?;
            }
        }

        Ok(Self {
            engine: Arc::new(env),
        })
    }

    #[pyo3(signature=(template_name, context=None))]
    fn render(
        &self,
        template_name: String,
        context: Option<Bound<'_, PyDict>>,
        py: Python<'_>,
    ) -> PyResult<String> {
        let template = to_py_exception(self.engine.get_template(&template_name))?;
        let mut ctx_values: HashMap<String, serde_json::Value> = HashMap::new();
        if let Some(context) = context {
            let serialize = crate::json::dumps(&context.into_py_any(py)?)?;
            ctx_values = to_py_exception(serde_json::from_str(&serialize))?;
        }
        to_py_exception(template.render(ctx_values))
    }
}
