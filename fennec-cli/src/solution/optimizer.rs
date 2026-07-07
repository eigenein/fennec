use std::{cmp::Ordering, range::RangeInclusive, time::Instant};

use chrono::{DateTime, Local};

use crate::{
    Schedule,
    Series,
    battery,
    battery::WorkingMode,
    energy,
    prelude::*,
    quantity::{
        Quantity,
        energy::{EnergyLevel, WattHours},
        power::Watts,
        price::{KilowattHourPrice, MillsPerHour},
        time::Hours,
    },
    series::Slot,
    solution::{Losses, Metrics, Solution, Space, Stage, Step},
};

#[must_use]
pub struct Optimizer {
    battery_capacity: WattHours,
    max_battery_flow: energy::Flow<Watts>,
    allowed_energy_levels: RangeInclusive<WattHours<usize>>,
    battery_degradation_cost: KilowattHourPrice,
    preferred_mode_bias: MillsPerHour,
    working_modes: Vec<WorkingMode>,
    energy_profile: energy::Profile,
    solution_space: Space,
}

impl Optimizer {
    pub fn new(
        energy_profile: energy::Profile,
        battery_args: &battery::Args,
        battery_capacity: WattHours,
        allowed_energy_levels: RangeInclusive<EnergyLevel>,
    ) -> Self {
        Self {
            battery_capacity,
            max_battery_flow: battery_args
                .power_limits
                .max_effective_flow(energy_profile.balance.eps_active_power.0),
            energy_profile,
            allowed_energy_levels,
            battery_degradation_cost: battery_args.degradation_cost,
            preferred_mode_bias: battery_args.preferred_mode_bias,
            working_modes: battery_args.working_modes.clone(),

            // TODO: this is better done by type state, but that would require forwarding the above args.
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
        allowed_energy_levels: RangeInclusive<EnergyLevel>,
    ) -> bool {
        (self.battery_capacity == battery_capacity)
            && (self.allowed_energy_levels == allowed_energy_levels)
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

        info!(?self.allowed_energy_levels, n_intervals = energy_prices.len(), "optimizing…");

        let capacity_level = EnergyLevel::from(self.battery_capacity);
        self.solution_space = energy_prices.map(|price| Stage::new(*price, capacity_level));

        // Going backwards:
        for interval_index in (0..self.solution_space.len()).rev() {
            // Calculate partial solutions for the current time interval:
            for energy_level in (0..=capacity_level.0).map(Quantity) {
                self.optimize_state(interval_index, energy_level, None);
            }
        }

        info!(elapsed = ?start_instant.elapsed(), "optimized");
    }

    /// Advance the optimizer solution space so that it starts at the specified timestamp.
    ///
    /// Returns [`true`] if and only if at least one interval got popped in the process.
    #[must_use]
    pub fn advance_to(&mut self, timestamp: DateTime<Local>) -> bool {
        let previous_len = self.solution_space.len();
        self.solution_space.advance_to(timestamp);
        self.solution_space.len() != previous_len
    }

    /// Optimize the state and assign the solution.
    ///
    /// If a preferred mode is specified, it wins over the optimal working mode when
    /// the gain is negligible.
    pub fn optimize_state(
        &mut self,
        interval_index: usize,
        initial_energy_level: EnergyLevel,
        preferred_working_mode: Option<WorkingMode>,
    ) {
        let Slot { interval, value: stage } = self.solution_space.get(interval_index);
        let duration = interval.duration().into();
        let average_balance = self.energy_profile.balance.mean_over(interval);
        let battery_simulator = battery::Simulator {
            residual_energy: initial_energy_level.into(),
            capacity: self.battery_capacity,
            efficiency: self.energy_profile.battery.efficiency,
        };
        self.solution_space.get_mut(interval_index)[initial_energy_level] = self
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

                if next_interval_index < self.solution_space.len() {
                    // For non-boundary solutions, accumulate the target optimization metrics:
                    metrics += self.solution_space.get(next_interval_index).value
                        [step.energy_level_after]
                        .as_ref()?
                        .metrics;
                }

                Some(Solution { metrics, step })
            })
            .min_by(|lhs, rhs| {
                if let Some(preferred_working_mode) = preferred_working_mode {
                    // If the solutions are very similar cost-wise, pick the preferred working mode:
                    let loss_diff_rate = ((lhs.total_loss() - rhs.total_loss()) / duration).abs();
                    if loss_diff_rate < self.preferred_mode_bias {
                        if lhs.step.working_mode == preferred_working_mode {
                            info!(preferred = ?preferred_working_mode, over = ?rhs.step.working_mode, ?loss_diff_rate, "picking");
                            return Ordering::Less;
                        }
                        if rhs.step.working_mode == preferred_working_mode {
                            info!(preferred = ?preferred_working_mode, over = ?lhs.step.working_mode, ?loss_diff_rate, "picking");
                            return Ordering::Greater;
                        }
                    }
                }
                lhs.compare_loss_to(rhs)
            });
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
            energy_level_after: battery.residual_energy.into(),
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
