use core::marker::PhantomData;

use bytes::BufMut;

use crate::protocol::{Request, codec};

/// Request encoder.
pub struct Encoder<T>(
    /// Binding to the arguments encoder type.
    PhantomData<T>,
);

impl<A, T: codec::Encoder<A>> codec::Encoder<Request<A>> for Encoder<T> {
    fn encode(request: &Request<A>, to: &mut impl BufMut) {
        to.put_u8(request.function_code);
        T::encode(&request.args, to);
    }
}
