use std::range::RangeInclusive;

use crate::{
    Schedule,
    api::mini_qube,
    battery,
    battery::WorkingMode,
    cli::BatteryConfigurationArgs,
    energy,
    energy::{Balance, Flow},
    quantity::{
        energy::{EnergyLevel, WattHours},
        power::Watts,
        price::KilowattHourPrice,
        time::Hours,
    },
    schedule::Slot,
    solution::{Losses, Metrics, Solution, Step, space::Stage},
};

pub struct StateOptimizer<'a> {
    battery_configuration: &'a BatteryConfigurationArgs,
    energy_profile: &'a energy::Profile,
    allowed_energy_levels: RangeInclusive<EnergyLevel>,
    battery_capacity: WattHours,
}

impl<'a> StateOptimizer<'a> {
    pub fn new(
        battery_configuration: &'a BatteryConfigurationArgs,
        battery_metrics: &'_ mini_qube::Metrics,
        energy_profile: &'a energy::Profile,
    ) -> Self {
        Self {
            battery_configuration,
            allowed_energy_levels: battery_metrics.allowed_energy_levels(),
            energy_profile,
            battery_capacity: battery_metrics.tracked.actual_capacity(),
        }
    }

    /// # Returns
    ///
    /// - [`Some`] [`PartialSolution`], if a solution exists.
    /// - [`None`], if there is no solution.
    pub fn optimize(
        &self,
        interval_index: usize,
        initial_energy_level: EnergyLevel,
        solutions: &Schedule<Stage>,
    ) -> Option<Solution> {
        let Slot { interval, value: stage } = solutions.get(interval_index);
        let duration = interval.duration();
        let average_balance = self.energy_profile.mean_balance_over(interval);
        let battery_simulator = battery::Simulator {
            residual_energy: initial_energy_level.into(),
            capacity: self.battery_capacity,
            efficiency: self.energy_profile.battery_efficiency,
        };
        let max_battery_flow = self
            .battery_configuration
            .power_limits
            .max_effective_flow(self.energy_profile.eps_active_power.0);
        self.battery_configuration
            .working_modes
            .iter()
            .filter_map(|working_mode| {
                let step = self.simulate(
                    battery_simulator,
                    duration,
                    average_balance,
                    stage.price(),
                    *working_mode,
                    max_battery_flow,
                );
                if (step.energy_level_after < initial_energy_level)
                    && (initial_energy_level <= self.allowed_energy_levels.start)
                {
                    // At or under the minimum allowed energy level, forbid going lower:
                    return None;
                }
                if (step.energy_level_after > initial_energy_level)
                    && (initial_energy_level >= self.allowed_energy_levels.last)
                {
                    // At or above the maximum allowed energy level, forbid going higher:
                    return None;
                }

                let mut metrics = step.metrics;
                let next_interval_index = interval_index + 1;

                if next_interval_index < solutions.len() {
                    // For non-boundary solutions, accumulate the target optimization metrics:
                    metrics += solutions.get(next_interval_index).value[step.energy_level_after]
                        .as_ref()?
                        .metrics;
                }

                Some(Solution { metrics, step })
            })
            .min()
    }

    /// Simulate the battery working in the specified mode given the initial conditions.
    fn simulate(
        &self,
        mut battery: battery::Simulator,
        duration: Hours,
        average_balance: Balance<Watts>,
        energy_price: Flow<KilowattHourPrice>,
        working_mode: WorkingMode,
        max_battery_flow: Flow<Watts>,
    ) -> Step {
        // Remember that the average flow represents theoretical possibility,
        // actual flow depends on the working mode:
        let balance_request = average_balance.with_working_mode(working_mode, max_battery_flow);

        let battery_flows = battery.apply(balance_request.battery, duration);
        let requested_battery = balance_request.battery * duration;
        let battery_shortage = requested_battery - battery_flows.external;
        let grid_flow = balance_request.grid * duration + battery_shortage.reversed();
        Step {
            working_mode,
            duration,
            energy_balance: Balance {
                grid: grid_flow.normalized(), // Normalize rare tiny negative values.
                battery: battery_flows.external,
            },
            energy_level_after: battery.residual_energy.into(),
            metrics: Metrics {
                internal_battery_flow: battery_flows.internal,
                losses: Losses {
                    grid: energy_price.loss(grid_flow),
                    battery: (battery_flows.internal.import + battery_flows.internal.export)
                        * self.battery_configuration.degradation_cost,
                },
            },
        }
    }
}
