use std::sync::Arc;

pub mod frank_energie;
pub mod homewizard;
pub mod mini_qube;

pub struct Connections {
    pub grid_measurement: homewizard::Client,
    pub battery: Arc<mini_qube::Client>,
}
