use crate::{
    quantity::{Zero, energy::WattHours, power::Watts, time::Hours},
    statistics::{battery::BatteryEfficiency, flow::Flow},
};

#[derive(Copy, Clone, bon::Builder)]
pub struct Simulator {
    /// Minimally allowed residual energy.
    ///
    /// This is normally calculated from the actual capacity and minimal state-of-charge setting.
    min_residual_energy: WattHours,

    /// Current residual energy.
    residual_energy: WattHours,

    /// Maximum allowed residual energy.
    max_residual_energy: WattHours,

    efficiency: BatteryEfficiency,
}

impl Simulator {
    pub const fn residual_energy(&self) -> WattHours {
        self.residual_energy
    }

    /// Apply the requested power, update the internal state and return actual billable energy flow.
    pub fn apply(&mut self, external_power: Flow<Watts>, for_: Hours) -> Flow<WattHours> {
        // Apply the efficiency corrections first – then, we can model everything in terms of residual energy:
        let internal_power = Flow {
            import: external_power.import * self.efficiency.charging,
            export: external_power.export / self.efficiency.discharging,
        };
        let requested_flow = internal_power * for_;
        let capacity = Flow {
            import: self.residual_energy.max(self.max_residual_energy) - self.residual_energy,
            export: self.residual_energy - self.residual_energy.min(self.min_residual_energy),
        };
        let mut actual_flow = Flow {
            import: requested_flow.import.min(capacity.import),
            export: requested_flow.export.min(capacity.export),
        };
        let declined_flow = requested_flow - actual_flow;
        if declined_flow.import >= declined_flow.export {
            // Import is more constrained, reclaim the capacity from the guaranteed export:
            actual_flow.import += declined_flow.import.min(actual_flow.export);
        } else {
            // Export is more constrained:
            actual_flow.export += declined_flow.export.min(actual_flow.import);
        }

        // Apply the net flow and correct on the parasitic load:
        self.residual_energy = self.residual_energy + actual_flow.import
            - actual_flow.export
            - self.efficiency.parasitic_load * for_;

        // Parasitic load may drain to the ground:
        self.residual_energy = self.residual_energy.max(WattHours::ZERO);

        // Convert the actual flow back to the external billable energy:
        Flow {
            import: actual_flow.import / self.efficiency.charging,
            export: actual_flow.export * self.efficiency.discharging,
        }
    }
}
