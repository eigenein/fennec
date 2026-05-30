use std::range::RangeInclusive;

use crate::{
    battery,
    energy::Flow,
    quantity::{Zero, energy::WattHours, power::Watts, time::Hours},
};

#[derive(Copy, Clone)]
pub struct Simulator {
    pub efficiency: battery::Efficiency,

    /// Allowed residual energy range.
    pub allowed_residual_energy: RangeInclusive<WattHours>,

    /// Current residual energy.
    pub residual_energy: WattHours,
}

impl Simulator {
    /// Apply the requested power, update the internal state and return actual billable energy flow.
    pub fn apply(&mut self, external_power: Flow<Watts>, for_: Hours) -> Flows {
        // Apply the efficiency corrections first – then, we can model everything in terms of residual energy:
        let internal_power = Flow {
            import: external_power.import * self.efficiency.charging,
            export: external_power.export / self.efficiency.discharging,
        };
        let requested_flow = internal_power * for_;
        let capacity = Flow {
            import: self.residual_energy.max(self.allowed_residual_energy.last)
                - self.residual_energy,
            export: self.residual_energy
                - self.residual_energy.min(self.allowed_residual_energy.start),
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

        Flows {
            external: Flow {
                // Convert the actual flow back to the external billable energy:
                import: actual_flow.import / self.efficiency.charging,
                export: actual_flow.export * self.efficiency.discharging,
            },
            internal: actual_flow,
        }
    }
}

pub struct Flows {
    pub external: Flow<WattHours>,
    pub internal: Flow<WattHours>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quantity::Quantity;

    /// Verify normal charging without overflowing.
    #[test]
    fn normal_operation() {
        let mut simulator = Simulator {
            residual_energy: Quantity(5000.0),
            allowed_residual_energy: (Zero::ZERO..=Quantity(10000.0)).into(),
            efficiency: battery::Efficiency::IDEAL,
        };
        let flows =
            simulator.apply(Flow { import: Watts(1000.0), export: Watts(700.0) }, Hours(1.0));
        assert_eq!(flows.external.import, Quantity(1000.0));
        assert_eq!(flows.external.export, Quantity(700.0));
        assert_eq!(simulator.residual_energy, Quantity(5300.0));
    }

    /// Verify efficiency corrections.
    #[test]
    fn efficiency() {
        let mut simulator = Simulator {
            residual_energy: Quantity(5000.0),
            allowed_residual_energy: (Zero::ZERO..=Quantity(10000.0)).into(),
            efficiency: battery::Efficiency {
                charging: 0.9,
                discharging: 0.5,
                parasitic_load: Watts(50.0),
            },
        };
        let flows =
            simulator.apply(Flow { import: Watts(1000.0), export: Watts(1000.0) }, Hours(1.0));
        assert_eq!(flows.external.import, Quantity(1000.0));
        assert_eq!(flows.external.export, Quantity(1000.0));
        assert_eq!(flows.internal.import, Quantity(900.0));
        assert_eq!(flows.internal.export, Quantity(2000.0));
        assert_eq!(
            simulator.residual_energy,
            Quantity(5000.0) + Quantity(1000.0) * simulator.efficiency.charging
                - Quantity(1000.0) / simulator.efficiency.discharging
                - simulator.efficiency.parasitic_load * Hours(1.0)
        );
    }

    /// Verify capping at the maximum.
    #[test]
    fn overflow() {
        let mut simulator = Simulator {
            residual_energy: Quantity(9000.0),
            allowed_residual_energy: (Zero::ZERO..=Quantity(10000.0)).into(),
            efficiency: battery::Efficiency::IDEAL,
        };
        let flows =
            simulator.apply(Flow { import: Watts(2000.0), export: Watts::ZERO }, Hours(1.0));
        assert_eq!(flows.external.import, Quantity(1000.0));
        assert_eq!(flows.external.export, Quantity::ZERO);
        assert_eq!(simulator.residual_energy, Quantity(10000.0));
    }

    /// Verify capping at the minimum.
    #[test]
    fn underflow() {
        let mut simulator = Simulator {
            residual_energy: Quantity(1000.0),
            allowed_residual_energy: (Quantity(500.0)..=Quantity(10000.0)).into(),
            efficiency: battery::Efficiency::IDEAL,
        };
        let flows =
            simulator.apply(Flow { import: Watts::ZERO, export: Watts(1000.0) }, Hours(1.0));
        assert_eq!(flows.external.import, Quantity::ZERO);
        assert_eq!(flows.external.export, Quantity(500.0));
        assert_eq!(simulator.residual_energy, Quantity(500.0));
    }

    /// Verify bidirectional operation at the minimum SoC.
    #[test]
    fn min_soc_bidirectional() {
        let mut simulator = Simulator {
            residual_energy: Quantity(100.0),
            allowed_residual_energy: (Quantity(100.0)..=Quantity(10000.0)).into(),
            efficiency: battery::Efficiency::IDEAL,
        };
        let flows =
            simulator.apply(Flow { import: Watts(500.0), export: Watts(1000.0) }, Hours(1.0));
        assert_eq!(flows.external.import, Quantity(500.0));
        assert_eq!(flows.external.export, Quantity(500.0));
        assert_eq!(simulator.residual_energy, Quantity(100.0));
    }

    /// Verify bidirectional operation at the maximum SoC.
    #[test]
    fn max_soc_bidirectional() {
        let mut simulator = Simulator {
            residual_energy: Quantity(10000.0),
            allowed_residual_energy: (Zero::ZERO..=Quantity(10000.0)).into(),
            efficiency: battery::Efficiency::IDEAL,
        };
        let flows =
            simulator.apply(Flow { import: Watts(1000.0), export: Watts(500.0) }, Hours(1.0));
        assert_eq!(flows.external.import, Quantity(500.0));
        assert_eq!(flows.external.export, Quantity(500.0));
        assert_eq!(simulator.residual_energy, Quantity(10000.0));
    }
}
