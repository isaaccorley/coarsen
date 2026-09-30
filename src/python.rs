//! The Python boundary owns bytes; all geometry work executes without the GIL.
use crate::{coverage_simplify, wkb};
use pyo3::{exceptions::PyValueError, prelude::*, types::PyBytes};
use rayon::prelude::*;

#[pyfunction]
#[pyo3(signature = (inputs, tolerance, simplify_boundary, threads=None))]
fn simplify_wkb<'py>(
    py: Python<'py>,
    inputs: Vec<Vec<u8>>,
    tolerance: f64,
    simplify_boundary: bool,
    threads: Option<usize>,
) -> PyResult<Vec<Bound<'py, PyBytes>>> {
    if threads == Some(0) {
        return Err(PyValueError::new_err("threads must be positive"));
    }
    let result = py
        .detach(move || -> anyhow::Result<Vec<Vec<u8>>> {
            let work = || -> anyhow::Result<Vec<Vec<u8>>> {
                let mut geometries = inputs
                    .par_iter()
                    .map(|b| wkb::read(b))
                    .collect::<anyhow::Result<Vec<_>>>()?;
                coverage_simplify(&mut geometries, tolerance, simplify_boundary);
                Ok(geometries.par_iter().map(wkb::write).collect())
            };
            match threads {
                Some(n) => rayon::ThreadPoolBuilder::new()
                    .num_threads(n)
                    .build()?
                    .install(work),
                None => work(),
            }
        })
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    Ok(result.iter().map(|b| PyBytes::new(py, b)).collect())
}

#[pymodule]
fn _core(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(simplify_wkb, module)?)?;
    Ok(())
}
