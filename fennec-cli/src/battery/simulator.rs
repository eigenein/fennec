use crate::{
    energy::Flow,
    quantity::{Zero, energy::WattHours, power::Watts, time::Hours},
};

#[derive(Copy, Clone)]
pub struct Simulator {
    pub efficiency: Flow<f64>,

    /// Battery capacity.
    ///
    /// This used to be range between minimum SoC and maximum SoC.
    /// But the battery rendered that moot since it did not always follow the limits.
    /// Hence, the simulator is just simulating.
    /// All the checks must be done at the solution space level.
    pub capacity: WattHours,

    /// Current residual energy.
    pub residual_energy: WattHours,
}

impl Simulator {
    /// Apply the requested power, update the internal state and return actual billable energy flow.
    pub fn apply(&mut self, external_power: Flow<Watts>, for_: Hours) -> Flows {
        // Apply the efficiency corrections first – then, we can model everything in terms of residual energy:
        let internal_power = Flow {
            import: external_power.import * self.efficiency.import,
            export: external_power.export / self.efficiency.export,
        };
        let requested_flow = internal_power * for_;
        let capacity = Flow {
            import: self.residual_energy.max(self.capacity) - self.residual_energy,
            export: self.residual_energy,
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

        // Apply the net flow:
        self.residual_energy += actual_flow.import - actual_flow.export;

        // Parasitic load may drain to the ground:
        self.residual_energy = self.residual_energy.max(WattHours::ZERO);

        Flows {
            external: Flow {
                // Convert the actual flow back to the external billable energy:
                import: actual_flow.import / self.efficiency.import,
                export: actual_flow.export * self.efficiency.export,
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
    use crate::quantity::{Quantity, Zero};

    const IDEAL_EFFICIENCY: Flow<f64> = Flow { import: 1.0, export: 1.0 };

    /// Verify normal charging without overflowing.
    #[test]
    fn normal_operation() {
        let mut simulator = Simulator {
            residual_energy: Quantity(5000.0),
            capacity: Quantity(10000.0),
            efficiency: IDEAL_EFFICIENCY,
        };
        let flows = simulator
            .apply(Flow { import: Quantity(1000.0), export: Quantity(700.0) }, Quantity(1.0));
        assert_eq!(flows.external.import, Quantity(1000.0));
        assert_eq!(flows.external.export, Quantity(700.0));
        assert_eq!(simulator.residual_energy, Quantity(5300.0));
    }

    /// Verify efficiency corrections.
    #[test]
    fn efficiency() {
        let mut simulator = Simulator {
            residual_energy: Quantity(5000.0),
            capacity: Quantity(10000.0),
            efficiency: Flow { import: 0.9, export: 0.5 },
        };
        let flows = simulator
            .apply(Flow { import: Quantity(1000.0), export: Quantity(1000.0) }, Quantity(1.0));
        assert_eq!(flows.external.import, Quantity(1000.0));
        assert_eq!(flows.external.export, Quantity(1000.0));
        assert_eq!(flows.internal.import, Quantity(900.0));
        assert_eq!(flows.internal.export, Quantity(2000.0));
        assert_eq!(
            simulator.residual_energy,
            Quantity(5000.0) + Quantity(1000.0) * simulator.efficiency.import
                - Quantity(1000.0) / simulator.efficiency.export
        );
    }

    /// Verify capping at the maximum.
    #[test]
    fn overflow() {
        let mut simulator = Simulator {
            residual_energy: Quantity(9000.0),
            capacity: Quantity(10000.0),
            efficiency: IDEAL_EFFICIENCY,
        };
        let flows =
            simulator.apply(Flow { import: Quantity(2000.0), export: Watts::ZERO }, Quantity(1.0));
        assert_eq!(flows.external.import, Quantity(1000.0));
        assert_eq!(flows.external.export, Quantity::ZERO);
        assert_eq!(simulator.residual_energy, Quantity(10000.0));
    }

    /// Verify capping at the minimum.
    #[test]
    fn underflow() {
        let mut simulator = Simulator {
            residual_energy: Quantity(1000.0),
            capacity: Quantity(10000.0),
            efficiency: IDEAL_EFFICIENCY,
        };
        let flows =
            simulator.apply(Flow { import: Watts::ZERO, export: Quantity(2000.0) }, Quantity(1.0));
        assert_eq!(flows.external.import, Quantity::ZERO);
        assert_eq!(flows.external.export, Quantity(1000.0));
        assert_eq!(simulator.residual_energy, Quantity(0.0));
    }

    /// Verify bidirectional operation near zero state-of-charge.
    #[test]
    fn min_soc_bidirectional() {
        let mut simulator = Simulator {
            residual_energy: Quantity(100.0),
            capacity: Quantity(10000.0),
            efficiency: IDEAL_EFFICIENCY,
        };
        let flows = simulator
            .apply(Flow { import: Quantity(500.0), export: Quantity(1000.0) }, Quantity(1.0));
        assert_eq!(flows.external.import, Quantity(500.0));
        assert_eq!(flows.external.export, Quantity(600.0));
        assert_eq!(simulator.residual_energy, Quantity(0.0));
    }

    /// Verify bidirectional operation at the maximum SoC.
    #[test]
    fn max_soc_bidirectional() {
        let mut simulator = Simulator {
            residual_energy: Quantity(10000.0),
            capacity: Quantity(10000.0),
            efficiency: IDEAL_EFFICIENCY,
        };
        let flows = simulator
            .apply(Flow { import: Quantity(1000.0), export: Quantity(500.0) }, Quantity(1.0));
        assert_eq!(flows.external.import, Quantity(500.0));
        assert_eq!(flows.external.export, Quantity(500.0));
        assert_eq!(simulator.residual_energy, Quantity(10000.0));
    }
}
