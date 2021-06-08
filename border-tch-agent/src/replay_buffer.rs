//! Replay buffer.
use std::marker::PhantomData;
use tch::{Device, Tensor};
mod base;
pub use base::{ReplayBuffer, TchBatch, TchBuffer};
use border_core::Shape;

/// Adds capability of constructing [Tensor] with a static method.
pub trait ZeroTensor {
    /// Constructs zero tensor.
    fn zeros(shape: &[i64]) -> Tensor;
}

impl ZeroTensor for u8 {
    fn zeros(shape: &[i64]) -> Tensor {
        Tensor::zeros(&shape, (tch::kind::Kind::Uint8, Device::Cpu))
    }
}

impl ZeroTensor for i32 {
    fn zeros(shape: &[i64]) -> Tensor {
        Tensor::zeros(&shape, (tch::kind::Kind::Int, Device::Cpu))
    }
}

impl ZeroTensor for f32 {
    fn zeros(shape: &[i64]) -> Tensor {
        Tensor::zeros(&shape, tch::kind::FLOAT_CPU)
    }
}

impl ZeroTensor for i64 {
    fn zeros(shape: &[i64]) -> Tensor {
        Tensor::zeros(&shape, (tch::kind::Kind::Int64, Device::Cpu))
    }
}

/// A buffer consisting of a [Tensor](tch::Tensor).
///
/// Type parameter `D` is the data type of the buffer and one of `u8` or `f32`.
/// S is the shape of the buffer, excepting the first dimension, which is for minibatch.
/// Type parameter `T` is the data stored in the buffer.
pub struct TchTensorBuffer<D, S, T>
where
    D: 'static + Copy + tch::kind::Element + ZeroTensor,
    S: Shape,
    T: Into<Tensor>,
{
    buf: Tensor,
    phantom: PhantomData<(D, S, T)>,
}

impl<D, S, T> TchBuffer for TchTensorBuffer<D, S, T>
where
    D: 'static + Copy + tch::kind::Element + ZeroTensor,
    S: Shape,
    T: Clone + Into<Tensor>,
{
    type Item = T;
    type SubBatch = Tensor;

    /// Creates a buffer.
    ///
    /// Input argument `_n_proc` is not used.
    /// TODO: remove n_procs
    fn new(capacity: usize, _n_procs: usize) -> Self {
        let mut shape: Vec<_> = S::shape().to_vec().iter().map(|e| *e as i64).collect();
        shape.insert(0, capacity as i64);
        let buf = D::zeros(shape.as_slice());

        Self {
            buf,
            phantom: PhantomData,
        }
    }

    fn push(&mut self, index: i64, item: &Self::Item) {
        let val: Tensor = item.clone().into();
        debug_assert_eq!(&val.size().as_slice()[1..], &self.buf.size()[1..]);

        // Not support vectorized environment for now
        debug_assert_eq!(val.size()[0], 1);
        self.buf.get(index).copy_(&val.squeeze1(0));
    }

    /// Creates minibatch.
    fn batch(&self, batch_indexes: &Tensor) -> Tensor {
        self.buf.index_select(0, &batch_indexes)
    }
}
