use bytes::{Buf, buf::Take};

/// A [`Buf`] adapter which limits the bytes read and drops any remaining bytes.
///
/// Useful for forward-compat with devices that pack extra bytes you don't care about.
pub struct DropRemaining<T: Buf>(pub Take<T>);

impl<T: Buf> Buf for DropRemaining<T> {
    fn remaining(&self) -> usize {
        self.0.remaining()
    }

    fn chunk(&self) -> &[u8] {
        self.0.chunk()
    }

    fn advance(&mut self, count: usize) {
        self.0.advance(count);
    }
}

impl<T: Buf> Drop for DropRemaining<T> {
    fn drop(&mut self) {
        self.0.advance(self.0.remaining());
    }
}
