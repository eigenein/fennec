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

    /// Read the contents of a contiguous block of holding registers in a remote device.
    #[expect(async_fn_in_trait)]
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, level = "trace"))]
    async fn read_holding_registers(
        &self,
        unit_id: Self::UnitId,
        starting_address: u16,
        n_registers: u16,
    ) -> Result<Vec<u16>, Self::Error> {
        #[cfg(feature = "tracing")]
        tracing::trace!(?unit_id, starting_address, n_registers, "reading holding registers…");

        let args = read_registers::Args::builder()
            .starting_address(starting_address)
            .n_registers(n_registers)
            .build()?;
        Ok(self.call::<ReadHoldingRegisters>(unit_id, args).await?.words)
    }

    /// Read the contents of a contiguous block of holding registers in a remote device.
    ///
    /// This is the same function as [`Self::read_holding_registers`] – but with the register count known at compile time.
    #[expect(async_fn_in_trait)]
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, level = "trace"))]
    async fn read_holding_registers_exact<const N: usize>(
        &self,
        unit_id: Self::UnitId,
        starting_address: u16,
    ) -> Result<[u16; N], Self::Error> {
        #[cfg(feature = "tracing")]
        tracing::trace!(?unit_id, starting_address, N, "reading holding registers…");

        let args = read_registers::ArgsExact::<N>::new(starting_address);
        Ok(self.call::<ReadHoldingRegistersExact<N>>(unit_id, args).await?.words)
    }
}
