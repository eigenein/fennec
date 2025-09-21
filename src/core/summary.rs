use crate::units::currency::Cost;

/// Optimization summary.
#[derive(Copy, Clone)]
pub struct Summary {
    pub net_loss: Cost,
    pub net_loss_without_battery: Cost,
}

impl Summary {
    pub fn profit(&self) -> Cost {
        // We expect that with the battery we lose less… 😅
        self.net_loss_without_battery - self.net_loss
    }
}
