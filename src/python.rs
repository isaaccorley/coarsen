//! The Python boundary owns bytes; all geometry work executes without the GIL.
use crate::{coverage_simplify, wkb};
use pyo3::{exceptions::PyValueError, prelude::*, types::PyBytes};
use rayon::prelude::*;

type RingPaths = Vec<Vec<usize>>;
type Simplified<'py> = (Bound<'py, PyBytes>, RingPaths);

#[pyfunction]
#[pyo3(signature = (inputs, tolerances, rings, threads=None))]
fn topology_wkb<'py>(
    py: Python<'py>,
    inputs: Vec<Vec<u8>>,
    tolerances: Vec<f64>,
    rings: Vec<RingPaths>,
    threads: Option<usize>,
) -> PyResult<Vec<Simplified<'py>>> {
    use crate::{simple_geom, topology};
    if inputs.len() != tolerances.len() || inputs.len() != rings.len() || threads == Some(0) {
        return Err(PyValueError::new_err("invalid batch lengths or threads"));
    }
    let result = py
        .detach(move || -> anyhow::Result<_> {
            let work = || {
                inputs
                    .par_iter()
                    .zip(&tolerances)
                    .zip(&rings)
                    .map(|((b, &tol), paths)| {
                        anyhow::ensure!(tol >= 0.0, "Tolerance must be non-negative");
                        let mut g = simple_geom::read(b)?;
                        for path in paths {
                            let mut part = &mut g;
                            for &i in path {
                                part = part
                                    .parts
                                    .get_mut(i)
                                    .ok_or_else(|| anyhow::anyhow!("invalid ring path"))?;
                            }
                            anyhow::ensure!(part.kind == 2, "ring path must reference a line");
                            part.kind = simple_geom::RING;
                        }
                        g.validate_index_coordinates()?;
                        let result = topology::simplify(g, tol);
                        let mut rings = Vec::new();
                        fn collect(
                            g: &simple_geom::Shape,
                            path: &mut Vec<usize>,
                            rings: &mut RingPaths,
                        ) {
                            if g.kind == simple_geom::RING {
                                rings.push(path.clone());
                            }
                            if g.kind >= 4 && g.kind <= 7 {
                                for (i, part) in g.parts.iter().enumerate() {
                                    path.push(i);
                                    collect(part, path, rings);
                                    path.pop();
                                }
                            }
                        }
                        collect(&result, &mut Vec::new(), &mut rings);
                        Ok((simple_geom::write(&result), rings))
                    })
                    .collect::<anyhow::Result<Vec<_>>>()
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
    Ok(result
        .into_iter()
        .map(|(b, r)| (PyBytes::new(py, &b), r))
        .collect())
}

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
    module.add_function(wrap_pyfunction!(topology_wkb, module)?)?;
    Ok(())
}
