//! Codecs for functions that write multiple coils or registers.

use core::marker::PhantomData;

use crate::protocol::function::size_argument;

pub struct ArgsEncoder<A, V, S>(
    /// Binding to the address type.
    PhantomData<A>,
    /// Binding to the value type.
    PhantomData<V>,
    /// Binding to the size type, normally [`size_argument::Bits`] or [`size_argument::Words`].
    PhantomData<S>,
);
