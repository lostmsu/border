use anyhow::Result;
use border_core::{
    record::{BufferedRecorder, Record},
    shape, util, Policy,
};
use border_py_gym_env::{
    PyGymEnv, PyGymEnvBuilder, PyGymEnvDiscreteAct, PyGymEnvDiscreteActRawFilter, PyGymEnvObs,
    PyGymEnvObsRawFilter,
};
use serde::Serialize;
use std::{convert::TryFrom, fs::File};

shape!(ObsShape, [4]);

type ObsFilter = PyGymEnvObsRawFilter<ObsShape, f64, f32>;
type ActFilter = PyGymEnvDiscreteActRawFilter;
type Obs = PyGymEnvObs<ObsShape, f64, f32>;
type Act = PyGymEnvDiscreteAct;
type Env = PyGymEnv<Obs, Act, ObsFilter, ActFilter>;

struct RandomPolicy {}

impl Policy<Env> for RandomPolicy {
    fn sample(&mut self, _: &Obs) -> Act {
        let v = fastrand::u32(..=1);
        Act::new(vec![v as i32])
    }
}

#[derive(Debug, Serialize)]
struct CartpoleRecord {
    episode: usize,
    step: usize,
    reward: f32,
    obs: Vec<f64>,
}

impl TryFrom<&Record> for CartpoleRecord {
    type Error = anyhow::Error;

    fn try_from(record: &Record) -> Result<Self> {
        Ok(Self {
            episode: record.get_scalar("episode")? as _,
            step: record.get_scalar("step")? as _,
            reward: record.get_scalar("reward")?,
            obs: record
                .get_array1("obs")?
                .iter()
                .map(|v| *v as f64)
                .collect(), // obs: Vec::from_iter(
                            //     record.get_array1("obs")?.iter().map(|v| *v as f64)
                            // )
        })
    }
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    fastrand::seed(42);

    let obs_filter = ObsFilter::default();
    let act_filter = ActFilter::default();
    let mut recorder = BufferedRecorder::new();
    let mut env = PyGymEnvBuilder::default()
        .build("CartPole-v0", obs_filter, act_filter)
        .unwrap();
    env.set_render(true);
    let mut policy = RandomPolicy {};

    util::eval_with_recorder(&mut env, &mut policy, 5, &mut recorder);

    // Vec<_> field in a struct does not support writing a header in csv crate, so disable it.
    let mut wtr = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(File::create("examples/model/random_cartpole_eval.csv")?);
    for record in recorder.iter() {
        wtr.serialize(CartpoleRecord::try_from(record)?)?;
    }

    Ok(())
}
