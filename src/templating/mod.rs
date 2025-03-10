use pyo3::{
    types::{PyModule, PyModuleMethods},
    Bound, PyResult,
};

mod tera;

pub fn templating_submodule(parent_module: &Bound<'_, PyModule>) -> PyResult<()> {
    let templating = PyModule::new(parent_module.py(), "templating").unwrap();
    templating.add_class::<self::tera::Tera>()?;
    parent_module.add_submodule(&templating)
}
