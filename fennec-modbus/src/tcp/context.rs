use alloc::{collections::VecDeque, vec::Vec};
use core::sync::atomic::AtomicU16;

use crate::Result;

/// Sans-IO Modbus-over-TCP client context.
#[derive(Default)]
pub struct Context {
    pub transaction_counter: AtomicU16,

    /// Frames to get sent.
    ///
    /// Per the guidelines, we shouldn't try and send them concatenated. 😢
    pub send_queue: VecDeque<Vec<u8>>,
}

impl Context {
    pub fn send(&mut self) -> Result {
        todo!()
    }
}
