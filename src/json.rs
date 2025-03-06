use pyo3::{
    types::{PyAnyMethods, PyDict, PyModule},
    Py, PyObject, PyResult, Python,
};

pub fn dumps(data: &PyObject) -> PyResult<String> {
    Python::with_gil(|py| {
        let orjson_module = PyModule::import(py, "orjson")?;
        let serialized_data = orjson_module
            .call_method1("dumps", (data,))?
            .call_method1("decode", ("utf-8",))?;
        serialized_data.extract()
    })
}

pub fn loads(data: &str) -> PyResult<Py<PyDict>> {
    Python::with_gil(|py| {
        let orjson_module = PyModule::import(py, "orjson")?;
        let deserialized_data = orjson_module.call_method1("loads", (data,))?;
        deserialized_data.extract()
    })
}
