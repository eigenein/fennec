use std::{range::RangeInclusive, time::Instant};

use chrono::{DateTime, Local};

use crate::{
    Schedule,
    Series,
    battery,
    battery::WorkingMode,
    energy,
    prelude::*,
    quantity::{Quantity, energy::WattHours, power::Watts, price::KilowattHourPrice, time::Hours},
    series::Slot,
    solution::{Losses, Metrics, Solution, Space, Stage, Step},
};

#[must_use]
pub struct Optimizer {
    /// Actual battery capacity.
    battery_capacity: WattHours,

    /// Maximum allowed battery flow.
    max_battery_flow: energy::Flow<Watts>,

    /// Allowed residual energy levels per the battery settings.
    allowed_residual_energy: RangeInclusive<WattHours<usize>>,

    /// Minimal residual energy required by the end of the price horizon.
    min_final_residual_energy: WattHours<usize>,

    /// Incurred costs per energy flow to and from the battery.
    battery_degradation_cost: KilowattHourPrice,

    /// Allowed working modes.
    working_modes: Vec<WorkingMode>,

    /// Learned energy profile to make battery usage prognoses.
    energy_profile: energy::Profile,

    /// Maintained solution space – this is what we are for.
    solution_space: Space,
}

impl Optimizer {
    pub fn new(
        energy_profile: energy::Profile,
        battery_args: &battery::Args,
        battery_capacity: WattHours,
        allowed_residual_energy: RangeInclusive<WattHours<usize>>,
        min_final_residual_energy: WattHours<usize>,
    ) -> Self {
        Self {
            battery_capacity,
            max_battery_flow: battery_args
                .power_limits
                .max_effective_flow(energy_profile.energy.eps_active_power.0),
            energy_profile,
            allowed_residual_energy,
            min_final_residual_energy,
            battery_degradation_cost: battery_args.degradation_cost,
            working_modes: battery_args.working_modes.clone(),
            solution_space: Series::new(),
        }
    }

    pub const fn solution_space(&self) -> &Space {
        &self.solution_space
    }

    /// Returns [`true`] if the optimizer's battery parameters still match – no rebuild needed.
    pub fn matches(
        &self,
        battery_capacity: WattHours,
        allowed_residual_energy: RangeInclusive<WattHours<usize>>,
    ) -> bool {
        (self.battery_capacity == battery_capacity) // FIXME: `f64` exact comparison.
            && (self.allowed_residual_energy == allowed_residual_energy)
    }

    /// Populate the solution space from scratch.
    ///
    /// Works backwards from future to present, computing the minimum cost at each
    /// `(timestamp, residual_energy)` state. Cost is money lost or gained to grid import or export.
    ///
    /// The [DP][1] state space:
    ///
    /// - Time dimension: each hour in the forecast period
    /// - Energy dimension: quantized with the specified step
    ///
    /// For each state, we pick the battery mode that minimizes total cost including future consequences.
    ///
    /// [1]: https://en.wikipedia.org/wiki/Dynamic_programming
    #[instrument(skip_all)]
    pub fn solve(&mut self, energy_prices: &Schedule<energy::Flow<KilowattHourPrice>>) {
        let start_instant = Instant::now();

        info!(?self.allowed_residual_energy, ?self.min_final_residual_energy, n_intervals = energy_prices.len(), "optimizing…");

        let battery_capacity: WattHours<usize> = self.battery_capacity.into();
        self.solution_space = energy_prices.map(|price| Stage::new(*price, battery_capacity));

        // Going backwards:
        for interval_index in (0..self.solution_space.len()).rev() {
            // Calculate partial solutions for the current time interval:
            for residual_energy in (0..=battery_capacity.0).map(Quantity) {
                self.optimize_state(interval_index, residual_energy);
            }
        }

        info!(elapsed = ?start_instant.elapsed(), "optimized");
    }

    /// Advance the optimizer solution space so that it starts at the specified timestamp.
    ///
    /// Returns [`true`] if and only if at least one interval got removed in the process.
    #[must_use]
    pub fn advance_to(&mut self, timestamp: DateTime<Local>) -> bool {
        self.solution_space.advance_to(timestamp) != 0
    }

    /// Optimize the state and assign the solution.
    pub fn optimize_state(
        &mut self,
        interval_index: usize,
        initial_residual_energy: WattHours<usize>,
    ) {
        let Slot { interval, value: stage } = self.solution_space.get(interval_index);
        let duration = interval.duration().into();
        let average_balance = self.energy_profile.energy.normalized_mean_over(interval);
        let battery_simulator = battery::Simulator {
            residual_energy: initial_residual_energy.into(),
            capacity: self.battery_capacity,
            efficiency: self.energy_profile.battery.efficiency,
        };
        self.solution_space.get_mut(interval_index)[initial_residual_energy] = self
            .working_modes
            .iter()
            .copied()
            .filter_map(|working_mode| {
                let step = self.simulate_step(
                    battery_simulator,
                    duration,
                    average_balance,
                    stage.price(),
                    working_mode,
                );
                if (step.residual_energy_after < initial_residual_energy)
                    && (initial_residual_energy <= self.allowed_residual_energy.start)
                {
                    // At or under the minimum allowed energy level, forbid going lower:
                    return None;
                }
                if (step.residual_energy_after > initial_residual_energy)
                    && (initial_residual_energy >= self.allowed_residual_energy.last)
                {
                    // At or above the maximum allowed energy level, forbid going higher:
                    return None;
                }

                let mut metrics = step.metrics;
                let next_interval_index = interval_index + 1;

                if next_interval_index < self.solution_space.len() {
                    // For non-boundary solutions, accumulate the target optimization metrics:
                    metrics += self.solution_space.get(next_interval_index).value
                        [step.residual_energy_after]
                        .as_ref()?
                        .metrics;
                } else if step.residual_energy_after < self.min_final_residual_energy {
                    // Enforce the final residual energy:
                    return None;
                }

                Some(Solution { metrics, step })
            })
            .min_by(Solution::compare_loss_to);
    }

    /// Simulate the battery working in the specified mode given the initial conditions.
    fn simulate_step(
        &self,
        mut battery: battery::Simulator,
        duration: Hours,
        average_balance: energy::Balance<Watts>,
        energy_price: energy::Flow<KilowattHourPrice>,
        working_mode: WorkingMode,
    ) -> Step {
        // Remember that the average flow represents theoretical possibility,
        // actual flow depends on the working mode:
        let balance_request =
            average_balance.with_working_mode(working_mode, self.max_battery_flow);

        let battery_flows = battery.apply(balance_request.battery, duration);
        let requested_battery = balance_request.battery * duration;
        let battery_shortage = requested_battery - battery_flows.external;
        let grid_flow = balance_request.grid * duration + battery_shortage.reversed();
        Step {
            working_mode,
            duration,
            energy_balance: energy::Balance {
                grid: grid_flow.normalized(), // Normalize rare tiny negative values.
                battery: battery_flows.external,
            },
            residual_energy_after: battery.residual_energy.into(),
            metrics: Metrics {
                internal_battery_flow: battery_flows.internal,
                losses: Losses {
                    grid: energy_price.loss(grid_flow),
                    battery: (battery_flows.internal.import + battery_flows.internal.export)
                        * self.battery_degradation_cost,
                },
            },
        }
    }
}
