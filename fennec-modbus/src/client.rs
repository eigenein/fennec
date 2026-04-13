use alloc::vec::Vec;
use core::fmt::Debug;

use crate::protocol::{
    Function,
    function::{ReadHoldingRegisters, ReadHoldingRegistersExact, read_registers},
};

/// Abstraction over async Modbus clients.
///
/// Concrete implementations get the provided shortcut functions for common operations.
pub trait AsyncClient {
    /// Server address type which allows to support proprietary node addressing like, for example, in Modbus+.
    type UnitId: Debug;

    type Error: From<crate::protocol::Error>;

    /// Call the Modbus function.
    ///
    /// This is a lower-level interface that allows calling any [`Function`], including user-defined ones.
    #[expect(async_fn_in_trait)]
    async fn call<F: Function>(
        &self,
        unit_id: Self::UnitId,
        args: F::Args,
    ) -> Result<F::Output, Self::Error>;

    /// Read the contents of a contiguous block of holding registers in a remote device
    /// and parse them as values of type `V`.
    #[expect(async_fn_in_trait)]
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, level = "trace"))]
    async fn read_holding_registers<V: read_registers::Value>(
        &self,
        unit_id: Self::UnitId,
        starting_address: u16,
        n_values: usize,
    ) -> Result<Vec<V>, Self::Error> {
        #[cfg(feature = "tracing")]
        tracing::trace!(?unit_id, starting_address, n_values, "reading holding registers…");

        let args = read_registers::Args::new(starting_address, n_values)?;
        Ok(self.call::<ReadHoldingRegisters<V>>(unit_id, args).await?.values)
    }

    /// Read the contents of a contiguous block of holding registers in a remote device
    /// and parse them as `N` values of type `V`.
    ///
    /// This is the same function as [`Self::read_holding_registers`] – but with the register count known at compile time.
    #[expect(async_fn_in_trait)]
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, level = "trace"))]
    async fn read_holding_registers_exact<const N: usize, V: read_registers::Value>(
        &self,
        unit_id: Self::UnitId,
        starting_address: u16,
    ) -> Result<[V; N], Self::Error> {
        #[cfg(feature = "tracing")]
        tracing::trace!(?unit_id, starting_address, N, "reading holding registers…");

        let args = read_registers::Args::new(starting_address, N)?;
        Ok(self.call::<ReadHoldingRegistersExact<N, V>>(unit_id, args).await?.values)
    }

    /// Convenience method to read a single value from one or more registers and unpack it.
    #[expect(async_fn_in_trait)]
    async fn read_holding_registers_value<V: read_registers::Value>(
        &self,
        unit_id: Self::UnitId,
        address: u16,
    ) -> Result<V, Self::Error> {
        let [value] = self.read_holding_registers_exact::<1, V>(unit_id, address).await?;
        Ok(value)
    }
}
