pub trait Zero {
    const ZERO: Self;
}

impl Zero for u16 {
    const ZERO: Self = 0;
}

impl Zero for i64 {
    const ZERO: Self = 0;
}

impl Zero for f64 {
    const ZERO: Self = 0.0;
}
