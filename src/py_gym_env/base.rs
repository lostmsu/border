#![allow(clippy::float_cmp)]
use std::borrow::Borrow;
use std::{fmt::Debug, error::Error};
use std::marker::PhantomData;
use log::{trace};
use pyo3::{PyObject, PyResult, Python, ToPyObject};
use pyo3::types::{PyTuple, IntoPyDict, PyList};
use crate::core::{Obs, Act, Info, Step, Env};

pub struct PyGymInfo {}

impl Info for PyGymInfo {}

/// Adapted from [tch-rs RL example](https://github.com/LaurentMazare/tch-rs/tree/master/examples/reinforcement-learning).
/// It represents non-vectorized environment (`n_procs`=1).
#[derive(Debug, Clone)]
pub struct PyGymEnv<O, A> {
    render: bool,
    env: PyObject,
    action_space: i64,
    observation_space: Vec<usize>,
    phantom: PhantomData<(O, A)>,
}

impl<O, A> PyGymEnv<O, A> where 
    O: Obs + From<PyObject>,
    A: Act + Into<PyObject> {
    pub fn new(name: &str) -> PyResult<Self> {
        let gil = Python::acquire_gil();
        let py = gil.python();

        // sys.argv is used by pyglet library, which is responsible for rendering.
        // Depending on the environment, however, sys.argv can be empty.
        // For that case, sys argv is set here.
        // See https://github.com/PyO3/pyo3/issues/1241#issuecomment-715952517
        let locals = [("sys", py.import("sys")?)].into_py_dict(py);
        let _ = py.eval("sys.argv.insert(0, 'PyGymEnv')", None, Some(&locals))?;

        let gym = py.import("gym")?;
        let env = gym.call("make", (name,), None)?;
        let _ = env.call_method("seed", (42,), None)?;
        let action_space = env.getattr("action_space")?;
        let action_space = if let Ok(val) = action_space.getattr("n") {
            val.extract()?
        } else {
            let action_space: Vec<i64> = action_space.getattr("shape")?.extract()?;
            action_space[0]
        };
        let observation_space = env.getattr("observation_space")?;
        let observation_space = observation_space.getattr("shape")?.extract()?;
        Ok(PyGymEnv {
            render: false,
            env: env.into(),
            action_space,
            observation_space,
            phantom: PhantomData,
        })
    }

    pub fn set_render(&mut self, render: bool) {
        self.render = render;
    }
}

fn pylist_to_act(py: &Python, a: PyObject) -> PyObject {
    let a_py_type = a.as_ref(*py).borrow().get_type().name().unwrap();
    if a_py_type == "list" {
        let l: &PyList = a.extract(*py).unwrap();
        l.get_item(0).into()
    }
    else {
        a
    }
}

impl<O, A> Env for PyGymEnv<O, A> where
    O: Obs + From<PyObject>,
    A: Act + Into<PyObject> + Debug {
    type Obs = O;
    type Act = A;
    type Info = PyGymInfo;

    /// Resets the environment, returning the observation tensor.
    /// In this environment, the length of `is_done` is assumed to be 1.
    fn reset(&self, is_done: Option<&Vec<f32>>) -> Result<O, Box<dyn Error>>  {
        match is_done {
            None => {
                pyo3::Python::with_gil(|py| {
                    let obs = self.env.call_method0(py, "reset")?;
                    Ok(obs.into())
                })
            },
            Some(v) => {
                if v[0] == 0.0 as f32 {
                    Ok(O::zero(1))
                }
                else {
                    pyo3::Python::with_gil(|py| {
                        let obs = self.env.call_method0(py, "reset")?;
                        Ok(obs.into())
                    })
                }
            }
        }
    }

    fn step(&self, a: &A) -> Step<Self> {
        trace!("{:?}", &a);
        pyo3::Python::with_gil(|py| {
            if self.render {
                let _ = self.env.call_method0(py, "render");
            }

            // Process action for continuous or discrete
            let a_py = a.clone().into();
            // let a_py = pylist_to_act(&py, a_py);

            let ret = self.env.call_method(py, "step", (a_py,), None).unwrap();
            let step: &PyTuple = ret.extract(py).unwrap();

            let obs = step.get_item(0).to_owned();
            let obs = obs.to_object(py).into();
            let reward: Vec<f32> = vec![step.get_item(1).extract().unwrap()];
            let is_done: Vec<f32> = vec![
                if step.get_item(2).extract().unwrap() {1.0} else {0.0}
            ];

            Step::<Self>::new(obs, a.clone(), reward, is_done, PyGymInfo{})
        })
    }
}
