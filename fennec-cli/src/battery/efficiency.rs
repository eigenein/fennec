use crate::quantity::power::Watts;

#[derive(Copy, Clone)]
pub struct Efficiency {
    pub charging: f64,
    pub discharging: f64,
    pub parasitic_load: Watts,
}
