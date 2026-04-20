//! PyO3 Python bindings for md2conf2md.

#[cfg(feature = "python")]
use pyo3::prelude::*;

#[cfg(feature = "python")]
use pythonize::{depythonize, pythonize};

#[cfg(feature = "python")]
use crate::adf::model::Document;

#[cfg(feature = "python")]
pyo3::create_exception!(_md2conf2md, ConversionError, pyo3::exceptions::PyValueError);

#[cfg(feature = "python")]
fn to_conversion_err(msg: impl ToString) -> PyErr {
    ConversionError::new_err(msg.to_string())
}

/// Convert Markdown text to an ADF document (returned as a Python dict).
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "md_to_adf")]
fn py_md_to_adf<'py>(py: Python<'py>, markdown: &str) -> PyResult<Bound<'py, PyAny>> {
    let doc = crate::md_to_adf::convert(markdown).map_err(to_conversion_err)?;
    pythonize(py, &doc).map_err(to_conversion_err)
}

/// Convert an ADF document (Python dict) to Markdown text.
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "adf_to_md")]
fn py_adf_to_md(py: Python<'_>, doc: &Bound<'_, PyAny>) -> PyResult<String> {
    let _ = py;
    let document: Document = depythonize(doc).map_err(to_conversion_err)?;
    crate::adf_to_md::convert(&document).map_err(to_conversion_err)
}

/// Convert Markdown text to ADF JSON string.
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "md_to_adf_json")]
fn py_md_to_adf_json(markdown: &str) -> PyResult<String> {
    crate::md_to_adf_json(markdown).map_err(to_conversion_err)
}

/// Convert ADF JSON string to Markdown text.
#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(name = "adf_json_to_md")]
fn py_adf_json_to_md(json: &str) -> PyResult<String> {
    crate::adf_json_to_md(json).map_err(to_conversion_err)
}

/// Register the Python module.
#[cfg(feature = "python")]
#[pymodule]
pub fn _md2conf2md(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("ConversionError", m.py().get_type::<ConversionError>())?;
    m.add_function(wrap_pyfunction!(py_md_to_adf, m)?)?;
    m.add_function(wrap_pyfunction!(py_adf_to_md, m)?)?;
    m.add_function(wrap_pyfunction!(py_md_to_adf_json, m)?)?;
    m.add_function(wrap_pyfunction!(py_adf_json_to_md, m)?)?;
    Ok(())
}
