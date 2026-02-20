use crate::{
    core::flow::Flow,
    quantity::{Zero, energy::WattHours, power::Watts, time::Hours},
    statistics::battery::BatteryEfficiency,
};

#[derive(Copy, Clone)]
pub struct Simulator {
    /// Minimally allowed residual energy.
    ///
    /// This is normally calculated from the actual capacity and minimal state-of-charge setting.
    pub min_residual_energy: WattHours,

    /// Current residual energy.
    pub residual_energy: WattHours,

    /// Maximum allowed residual energy.
    pub max_residual_energy: WattHours,

    pub efficiency: BatteryEfficiency,
}

impl Simulator {
    /// Apply the requested power, update the internal state and return actual billable energy flow.
    pub fn apply(&mut self, external_power: Flow<Watts>, for_: Hours) -> Simulation {
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
        Simulation {
            flow: Flow {
                import: actual_flow.import / self.efficiency.charging,
                export: actual_flow.export * self.efficiency.discharging,
            },
        }
    }
}

pub struct Simulation {
    pub flow: Flow<WattHours>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify normal charging without overflowing.
    #[test]
    fn normal_operation() {
        let mut simulator = Simulator {
            residual_energy: WattHours(5000.0),
            min_residual_energy: WattHours::ZERO,
            max_residual_energy: WattHours(10000.0),
            efficiency: BatteryEfficiency::IDEAL,
        };
        let simulation =
            simulator.apply(Flow { import: Watts(1000.0), export: Watts::ZERO }, Hours(1.0));
        assert_eq!(simulation.flow.import, WattHours(1000.0));
        assert_eq!(simulation.flow.export, WattHours::ZERO);
        assert_eq!(simulator.residual_energy, WattHours(6000.0));
    }

    /// Verify efficiency corrections.
    #[test]
    fn efficiency() {
        let efficiency = BatteryEfficiency {
            parasitic_load: Watts(50.0),
            charging: 0.9,
            discharging: 0.5,
            n_samples: 0,
            total_hours: Hours::ZERO,
        };
        let mut simulator = Simulator {
            residual_energy: WattHours(5000.0),
            min_residual_energy: WattHours::ZERO,
            max_residual_energy: WattHours(10000.0),
            efficiency,
        };
        let simulation =
            simulator.apply(Flow { import: Watts(1000.0), export: Watts(1000.0) }, Hours(1.0));
        assert_eq!(simulation.flow.import, WattHours(1000.0));
        assert_eq!(simulation.flow.export, WattHours(1000.0));
        assert_eq!(
            simulator.residual_energy,
            WattHours(5000.0) + WattHours(1000.0) * efficiency.charging
                - WattHours(1000.0) / efficiency.discharging
                - efficiency.parasitic_load * Hours(1.0)
        );
    }

    /// Verify capping at the maximum.
    #[test]
    fn overflow() {
        let mut simulator = Simulator {
            residual_energy: WattHours(9000.0),
            min_residual_energy: WattHours::ZERO,
            max_residual_energy: WattHours(10000.0),
            efficiency: BatteryEfficiency::IDEAL,
        };
        let simulation =
            simulator.apply(Flow { import: Watts(2000.0), export: Watts::ZERO }, Hours(1.0));
        assert_eq!(simulation.flow.import, WattHours(1000.0));
        assert_eq!(simulation.flow.export, WattHours::ZERO);
        assert_eq!(simulator.residual_energy, WattHours(10000.0));
    }

    /// Verify capping at the minimum.
    #[test]
    fn underflow() {
        let mut simulator = Simulator {
            residual_energy: WattHours(1000.0),
            min_residual_energy: WattHours(500.0),
            max_residual_energy: WattHours(10000.0),
            efficiency: BatteryEfficiency::IDEAL,
        };
        let simulation =
            simulator.apply(Flow { import: Watts::ZERO, export: Watts(1000.0) }, Hours(1.0));
        assert_eq!(simulation.flow.import, WattHours::ZERO);
        assert_eq!(simulation.flow.export, WattHours(500.0));
        assert_eq!(simulator.residual_energy, WattHours(500.0));
    }

    /// Verify bidirectional operation at the minimum SoC.
    #[test]
    fn min_soc_bidirectional() {
        let mut simulator = Simulator {
            residual_energy: WattHours(100.0),
            min_residual_energy: WattHours(100.0),
            max_residual_energy: WattHours(10000.0),
            efficiency: BatteryEfficiency::IDEAL,
        };
        let simulation =
            simulator.apply(Flow { import: Watts(500.0), export: Watts(1000.0) }, Hours(1.0));
        assert_eq!(simulation.flow.import, WattHours(500.0));
        assert_eq!(simulation.flow.export, WattHours(500.0));
        assert_eq!(simulator.residual_energy, WattHours(100.0));
    }

    /// Verify bidirectional operation at the maximum SoC.
    #[test]
    fn max_soc_bidirectional() {
        let mut simulator = Simulator {
            residual_energy: WattHours(10000.0),
            min_residual_energy: WattHours(0.0),
            max_residual_energy: WattHours(10000.0),
            efficiency: BatteryEfficiency::IDEAL,
        };
        let simulation =
            simulator.apply(Flow { import: Watts(1000.0), export: Watts(500.0) }, Hours(1.0));
        assert_eq!(simulation.flow.import, WattHours(500.0));
        assert_eq!(simulation.flow.export, WattHours(500.0));
        assert_eq!(simulator.residual_energy, WattHours(10000.0));
    }
}
