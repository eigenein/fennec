use crate::quantity::{cost::Cost, energy::KilowattHours};

/// Solution summary.
#[derive(Copy, Clone)]
pub struct Summary {
    pub net_loss: Cost,
    pub net_loss_without_battery: Cost,
    pub peak_grid_consumption: KilowattHours,
}

impl Summary {
    pub fn profit(&self) -> Cost {
        // We expect that with the battery we lose less… 😅
        self.net_loss_without_battery - self.net_loss
    }
}
