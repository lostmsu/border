//! This module offers types for recording values obtained during training and evaluation.
//!
use std::{
    collections::{HashMap, hash_map::{Iter, IntoIter, Keys}},
    convert::Into, path::Path, iter::IntoIterator
};
use chrono::prelude::{DateTime, Local};
use ndarray::Array1;
use tensorboard_rs::summary_writer::SummaryWriter;

use crate::error::LrrError;

#[derive(Debug, Clone)]
/// Represents possible types of values in a [`Record`].
pub enum RecordValue {
    /// Represents a scalar, e.g., optimization steps and loss value.
    Scalar(f32),

    /// Represents a datetime.
    DateTime(DateTime<Local>),

    /// A 1-dimensional array
    Array1(Array1<f32>),
}

#[derive(Debug)]
/// Represents a record.
pub struct Record (HashMap<String, RecordValue>);

impl Record {
    /// Construct empty record.
    pub fn empty() -> Self {
        Self { 0: HashMap::new() }
    }

    /// Create `Record` from slice of `(Into<String>, RecordValue)`.
    pub fn from_slice<K: Into<String> + Clone>(s: &[(K, RecordValue)]) -> Self {
        Self (s.iter().map(|(k, v)| (k.clone().into(), v.clone())).collect())
    }

    /// Get keys.
    pub fn keys(&self) -> Keys<String, RecordValue> {
        self.0.keys()
    }

    /// Insert a key-value pair into the record.
    pub fn insert(&mut self, k: impl Into<String>, v: RecordValue) {
        self.0.insert(k.into(), v);
    }

    /// Return an iterator over key-value pairs in the record.
    pub fn iter(&self) -> Iter<'_, String, RecordValue> {
        self.0.iter()
    }

    /// Return an iterator over key-value pairs in the record.
    pub fn into_iter_in_record(self) -> IntoIter<String, RecordValue> {
        self.0.into_iter()
    }

    /// Get the value of the given key.
    pub fn get(&self, k: &str) -> Option<&RecordValue> {
        self.0.get(k)
    }

    /// Merge records.
    pub fn merge(self, record: Record) -> Self {
        Record(self.0.into_iter().chain(record.0).collect())
    }

    /// Get scalar value.
    pub fn get_scalar(&self, k: &str) -> Result<f32, LrrError> {
        if let Some(v) = self.0.get(k) {
            match v {
                RecordValue::Scalar(v) => Ok(*v as _),
                _ => Err(LrrError::RecordValueTypeError("Scalar".to_string()))
            }
        }
        else {
            Err(LrrError::RecordKeyError(k.to_string()))
        }
    }
}

/// Process records provided with [`Recorder::write`]
pub trait Recorder {
    /// Write a record to the [`Recorder`].
    fn write(&mut self, record: Record);
}

/// A recorder that ignores any record. This struct is used just for debugging.
pub struct NullRecorder {}

impl NullRecorder {}

impl Recorder for NullRecorder {
    /// Discard the given record.
    fn write(&mut self, _record: Record) {}
}

/// Write records to TFRecord.
pub struct TensorboardRecorder {
    writer: SummaryWriter,
    step_key: String,
}

impl TensorboardRecorder {
    /// Construct a [`TensorboardRecorder`].
    ///
    /// TFRecord will be stored in `logdir`.
    pub fn new<P: AsRef<Path>>(logdir: P) -> Self {
        Self {
            writer: SummaryWriter::new(logdir),
            step_key: "n_opts".to_string()
        }
    }
}

impl Recorder for TensorboardRecorder {
    /// Write a given [`Record`] into a TFRecord.
    ///
    /// It ignores [`RecordValue::Array1`]
    fn write(&mut self, record: Record) {
        // TODO: handle error
        let step = match record.get(&self.step_key).unwrap() {
            RecordValue::Scalar(v) => { *v as usize },
            _ => { panic!() }
        };

        for (k, v) in record.iter() {
            if *k != self.step_key {
                match v {
                    RecordValue::Scalar(v) => {
                        self.writer.add_scalar(k, *v as f32, step)
                    },
                    RecordValue::DateTime(_) => {}, // discard value
                    _ => { unimplemented!() }
                };                
            }
        }
    }
}

/// Buffered recorder.
///
/// This is used for recording sequences of observation and action
/// during evaluation runs in [`crate::core::util::eval_with_recorder`].
#[derive(Default)]
pub struct BufferedRecorder (Vec<Record>);

impl BufferedRecorder {
    /// Construct the recorder.
    pub fn new() -> Self { Self(Vec::default()) }

    /// Returns an iterator over the records.
    pub fn iter(&self) -> std::slice::Iter<Record> {
        self.0.iter()
    }
}

impl Recorder for BufferedRecorder {
    /// Write a [`Record`] to the buffer.
    ///
    /// TODO: Consider if it is worth making the method public.
    fn write(&mut self, record: Record) {
        self.0.push(record);
    }
}
