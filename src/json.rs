use pyo3::{
    types::{PyAnyMethods, PyDict, PyModule},
    Py, PyObject, PyResult, Python,
};

pub fn dumps(data: &PyObject) -> PyResult<String> {
    Python::with_gil(|py| -> PyResult<String> {
        let json = PyModule::import(py, "orjson")?;
        let json_data = json
            .call_method1("dumps", (data,))?
            .call_method1("decode", ("utf-8",))?;
        json_data.extract()
    })
}

pub fn loads(data: &str) -> PyResult<Py<PyDict>> {
    Python::with_gil(|py| -> PyResult<Py<PyDict>> {
        let json = PyModule::import(py, "orjson")?;
        let json_data = json.call_method1("loads", (data,))?;
        json_data.extract()
    })
}
