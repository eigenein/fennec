mod battery;
pub mod conditions;
mod energy;
pub mod solution;
pub mod step;

use std::{iter::from_fn, ops::Range, rc::Rc};

use bon::{Builder, bon, builder};
use chrono::{DateTime, Local, TimeDelta};
use enumset::EnumSet;
use itertools::Itertools;
use ordered_float::OrderedFloat;

use crate::{
    cli::BatteryArgs,
    core::{
        series::Point,
        solver::{
            battery::Battery,
            conditions::Conditions,
            energy::WattHours,
            solution::Solution,
            step::Step,
        },
        working_mode::WorkingMode,
    },
    prelude::*,
    quantity::{cost::Cost, energy::KilowattHours, power::Kilowatts, rate::KilowattHourRate},
};

#[derive(Builder)]
#[builder(finish_fn(vis = ""))]
pub struct Solver<'a> {
    conditions: &'a [(Range<DateTime<Local>>, Conditions)],
    working_modes: EnumSet<WorkingMode>,
    residual_energy: KilowattHours,
    capacity: KilowattHours,
    battery_args: BatteryArgs,
    purchase_fee: KilowattHourRate,
    now: DateTime<Local>,
}

impl<S: solver_builder::IsComplete> SolverBuilder<'_, S> {
    pub fn solve(self) -> Option<Solution> {
        self.build().solve()
    }
}

#[bon]
impl Solver<'_> {
    /// Find the optimal battery schedule.
    ///
    /// Works backwards from future to present, computing the minimum cost at each
    /// `(timestamp, residual_energy)` state. Cost is money lost or gained to grid import or export.
    ///
    /// The [DP][1] state space:
    ///
    /// - Time dimension: each hour in the forecast period
    /// - Energy dimension: quantized to 10 Wh increments (decawatt-hours)
    ///
    /// For each state, we pick the battery mode that minimizes total cost including future consequences.
    ///
    /// [1]: https://en.wikipedia.org/wiki/Dynamic_programming
    #[instrument(skip_all)]
    fn solve(self) -> Option<Solution> {
        let min_residual_energy =
            self.capacity * (f64::from(self.battery_args.min_soc_percent) / 100.0);
        let max_energy = WattHours::from(self.residual_energy.max(self.capacity));
        info!(?min_residual_energy, ?self.residual_energy, ?max_energy, "Optimizing…");

        // This is calculated in order to estimate the net profit:
        let mut net_loss_without_battery = Cost::ZERO;

        // Since we're going backwards in time, we only need to store the next hour's partial solutions
        // to find the current hour's solutions.
        //
        // They are wrapped in `Rc`, because the vector is going to be replaced every hour,
        // but we still need to backtrack the entire solution path.
        let mut next_partial_solutions = (0..=usize::from(max_energy))
            .map(|_| PartialSolution::new())
            .map(Rc::new)
            .map(Some)
            .collect_vec();

        // Going backwards:
        for (time_range, conditions) in self.conditions.iter().rev() {
            let step_duration = if time_range.contains(&self.now) {
                time_range.end - self.now
            } else {
                TimeDelta::hours(1)
            };

            // Average stand-by power at this hour of a day:
            net_loss_without_battery +=
                self.loss(conditions.grid_rate, conditions.stand_by_power * step_duration);

            // Calculate partial solutions for the current hour:
            next_partial_solutions = (0..=max_energy.0)
                .map(|initial_residual_energy_watt_hours| {
                    self.optimise_step()
                        .time_range(time_range.clone())
                        .conditions(conditions)
                        .initial_residual_energy(KilowattHours::from(WattHours(
                            initial_residual_energy_watt_hours,
                        )))
                        .min_residual_energy(min_residual_energy)
                        .next_partial_solutions(&next_partial_solutions)
                        .max_energy(max_energy)
                        .duration(step_duration)
                        .call()
                        .map(Rc::new)
                })
                .collect();
        }

        // By this moment, «next hour losses» is actually the upcoming hour, so our solution starts with:
        let initial_energy = WattHours::from(self.residual_energy);
        let initial_partial_solution =
            next_partial_solutions.into_iter().nth(usize::from(initial_energy)).unwrap()?;

        let solution = Solution {
            net_loss: initial_partial_solution.net_loss,
            net_loss_without_battery,
            steps: initial_partial_solution.backtrack().collect(),
        };
        info!(
            net_loss = ?solution.net_loss,
            without_battery = ?solution.net_loss_without_battery,
            profit = ?solution.profit(),
            "Optimized",
        );
        Some(solution)
    }

    /// # Returns
    ///
    /// - [`Some`] [`PartialSolution`], if a solution exists.
    /// - [`None`], if there is no solution.
    #[builder]
    fn optimise_step(
        &self,
        time_range: Range<DateTime<Local>>,
        conditions: &Conditions,
        initial_residual_energy: KilowattHours,
        min_residual_energy: KilowattHours,
        next_partial_solutions: &[Option<Rc<PartialSolution>>],
        max_energy: WattHours,
        duration: TimeDelta,
    ) -> Option<PartialSolution> {
        let battery = Battery::builder()
            .residual_energy(initial_residual_energy)
            .min_residual_energy(min_residual_energy)
            .capacity(self.capacity)
            .args(self.battery_args)
            .build();
        self.working_modes
            .iter()
            .filter_map(|working_mode| {
                let step = self
                    .simulate_step()
                    .conditions(conditions)
                    .initial_residual_energy(initial_residual_energy)
                    .battery(battery.clone())
                    .working_mode(working_mode)
                    .duration(duration)
                    .call();
                let next_partial_solution = {
                    let next_energy = WattHours::from(step.residual_energy_after).min(max_energy);
                    next_partial_solutions[usize::from(next_energy)].clone()
                }?;
                if step.residual_energy_after >= min_residual_energy {
                    Some(PartialSolution {
                        net_loss: step.loss + next_partial_solution.net_loss,
                        next: Some(next_partial_solution),
                        step: Some((time_range.clone(), step)),
                    })
                } else {
                    // Do not allow dropping below the minimally allowed state-of-charge:
                    None
                }
            })
            .min_by_key(|partial_solution| {
                // TODO: make `Quantity` orderable:
                OrderedFloat(partial_solution.net_loss.0)
            })
    }

    /// Simulate the battery working in the specified mode given the initial conditions,
    /// and return the loss and new residual energy.
    #[builder]
    fn simulate_step(
        &self,
        mut battery: Battery,
        conditions: &Conditions,
        initial_residual_energy: KilowattHours,
        working_mode: WorkingMode,
        duration: TimeDelta,
    ) -> Step {
        // Requested external power flow to (positive) or from (negative) the battery:
        let battery_external_power = match working_mode {
            WorkingMode::Idle => Kilowatts::ZERO,
            WorkingMode::Backup => (-conditions.stand_by_power).max(Kilowatts::ZERO),
            WorkingMode::ChargeSlowly => self.battery_args.charging_power * 0.5,
            WorkingMode::Charge => self.battery_args.charging_power,
            WorkingMode::Discharge => -self.battery_args.discharging_power,
            WorkingMode::Balance => (-conditions.stand_by_power)
                .clamp(-self.battery_args.discharging_power, self.battery_args.charging_power),
        };

        // Apply the load to the battery:
        let battery_active_time = battery.apply_load(battery_external_power, duration);

        // Total household energy balance:
        let grid_consumption =
            battery_external_power * battery_active_time + conditions.stand_by_power * duration;

        Step {
            working_mode,
            residual_energy_before: initial_residual_energy,
            residual_energy_after: battery.residual_energy(),
            grid_consumption,
            loss: self.loss(conditions.grid_rate, grid_consumption),
        }
    }

    /// Calculate the grid consumption or production loss.
    fn loss(&self, grid_rate: KilowattHourRate, consumption: KilowattHours) -> Cost {
        if consumption >= KilowattHours::ZERO {
            consumption * grid_rate
        } else {
            // We sell excess energy cheaper:
            consumption * (grid_rate - self.purchase_fee)
        }
    }
}

/// TODO: this could be named just `Solution` – and potentially, the original `Solution` could be gone.
struct PartialSolution {
    /// Net loss from the current state till the forecast period end – our primary optimization target.
    net_loss: Cost,

    /// Next partial solution – allows backtracking the entire sequence.
    ///
    /// I use [`Rc`] here to avoid storing the entire state matrix. That way, I calculate hour by
    /// hour, while moving from the future to the present. When all the states for the current hour
    /// are calculated, I can safely drop the previous hour states, because I keep the relevant
    /// links via [`Rc`].
    ///
    /// TODO: these two [`Option`] attributes are linked, so join them.
    next: Option<Rc<PartialSolution>>,

    /// The current step metrics.
    ///
    /// Technically, it is not needed to store the timestamp here because I could always zip
    /// the back track with the original metrics, but having it here makes it much easier to work with
    /// (and to ensure it is working properly).
    step: Option<(Range<DateTime<Local>>, Step)>,
}

impl PartialSolution {
    pub const fn new() -> Self {
        Self { net_loss: Cost::ZERO, next: None, step: None }
    }

    /// Track the optimal solution till the end.
    fn backtrack(&self) -> impl Iterator<Item = Point<Range<DateTime<Local>>, Step>> {
        let mut pointer = Some(self);
        from_fn(move || {
            // I'll need to yield the current step, so clone:
            let current_solution = pointer?;
            // …and advance:
            pointer = current_solution.next.as_deref();
            current_solution.step.clone() // TODO: can I do without `clone()`?
        })
    }
}
