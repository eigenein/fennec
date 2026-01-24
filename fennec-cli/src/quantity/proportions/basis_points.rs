use derive_more::{From, Into};

#[derive(Copy, Clone, From, Into)]
#[into(i64)]
pub struct BasisPoints(u16);
