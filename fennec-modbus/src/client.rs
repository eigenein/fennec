use alloc::vec::Vec;
use core::fmt::Debug;

use crate::{
    protocol,
    protocol::{
        BitSize,
        Decode,
        Function,
        function,
        function::{ReadRegisters, read_registers},
    },
};

/// Abstraction over async Modbus clients.
///
/// Concrete implementations get the provided shortcut functions for common operations.
pub trait AsyncClient {
    /// Server address type which allows to support proprietary node addressing like, for example, in Modbus+.
    type UnitId: Debug;

    type Error: From<protocol::Error>;

    /// Call the Modbus function.
    ///
    /// This is a lower-level interface that allows calling any [`Function`], including user-defined ones.
    #[expect(async_fn_in_trait)]
    async fn call<F: Function>(
        &self,
        unit_id: Self::UnitId,
        args: F::Args,
    ) -> Result<F::Output, Self::Error>;

    /// Read the contents of a contiguous block of registers in a remote device
    /// and parse them as values of type `V`.
    #[expect(async_fn_in_trait)]
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, level = "trace"))]
    async fn read_registers<C: function::Code, V: Decode + BitSize>(
        &self,
        unit_id: Self::UnitId,
        starting_address: u16,
        n_values: usize,
    ) -> Result<Vec<V>, Self::Error> {
        #[cfg(feature = "tracing")]
        tracing::trace!(?unit_id, starting_address, n_values, "reading holding registers…");

        let args = read_registers::Args::new(starting_address, n_values)?;
        Ok(self.call::<ReadRegisters<C, V>>(unit_id, args).await?.0)
    }
}
