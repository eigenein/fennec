#[derive(Copy, Clone, derive_more::From)]
pub struct Percent(u16);

impl Percent {
    pub const fn to_proportion(self) -> f64 {
        0.01 * self.0 as f64
    }
}
