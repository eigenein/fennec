mod battery;
mod energy;
pub mod step;

use std::{iter::from_fn, rc::Rc, time::Instant};

use bon::{Builder, bon};
use chrono::{DateTime, Local, Timelike};
use enumset::EnumSet;
use itertools::Itertools;
use quantities::{
    Quantity,
    cost::Cost,
    energy::KilowattHours,
    power::Kilowatts,
    rate::KilowattHourRate,
};

use crate::{
    cli::BatteryArgs,
    core::{
        interval::Interval,
        solver::{battery::Battery, energy::WattHours, step::Step},
        working_mode::WorkingMode,
    },
    prelude::*,
    statistics::energy::BatteryParameters,
};

#[derive(Builder)]
#[builder(finish_fn(vis = ""))]
pub struct Solver<'a> {
    grid_rates: &'a [(Interval, KilowattHourRate)],
    hourly_stand_by_power: &'a [Option<Kilowatts>; 24],
    working_modes: EnumSet<WorkingMode>,
    initial_residual_energy: KilowattHours,
    capacity: KilowattHours,
    battery_args: BatteryArgs,
    battery_parameters: BatteryParameters,
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
        let start_instant = Instant::now();
        let min_residual_energy = self.capacity * self.battery_args.min_soc();
        let max_energy = WattHours::from(self.initial_residual_energy.max(self.capacity));
        info!(
            ?min_residual_energy,
            initial_residual_energy = ?self.initial_residual_energy,
            ?max_energy,
            n_intervals = self.grid_rates.len(),
            "Optimizing…",
        );

        // This is calculated in order to estimate the net profit:
        let mut net_loss_without_battery = Cost::ZERO;

        // Since we're going backwards in time, we only need to store the next hour's partial solutions
        // to find the current hour's solutions.
        // Here, `None` means there's no solution for the respective residual energy.
        let mut solutions = vec![Some(Solution::new()); max_energy.0 as usize + 1];

        // Going backwards:
        for (mut interval, grid_rate) in self.grid_rates.iter().rev().copied() {
            if interval.contains(self.now) {
                // The interval has already started, trim the start time:
                interval = interval.with_start(self.now);
            }

            // Average stand-by power at this hour of a day:
            let stand_by_power = self.hourly_stand_by_power[interval.start.hour() as usize]
                .unwrap_or(Kilowatts::ZERO);
            net_loss_without_battery += self.loss(grid_rate, stand_by_power * interval.duration());

            // Calculate partial solutions for the current hour:
            solutions = {
                // Solutions from the past iteration become «next» in relation to the current step.
                // They are wrapped in `Rc`, because we're replacing the vector,
                // but we still need to backtrack the entire solution path.
                let next_solutions =
                    solutions.into_iter().map(|solution| solution.map(Rc::new)).collect_vec();
                (0..=max_energy.0)
                    .map(|initial_residual_energy_watt_hours| {
                        self.optimise_step()
                            .interval(interval)
                            .stand_by_power(stand_by_power)
                            .grid_rate(grid_rate)
                            .initial_residual_energy(KilowattHours::from(WattHours(
                                initial_residual_energy_watt_hours,
                            )))
                            .min_residual_energy(min_residual_energy)
                            .next_solutions(&next_solutions)
                            .max_energy(max_energy)
                            .call()
                    })
                    .collect_vec()
            };
        }

        // By this moment, «next hour losses» is actually the upcoming hour, so our solution starts with:
        let initial_energy = WattHours::from(self.initial_residual_energy);
        let solution = solutions.into_iter().nth(usize::from(initial_energy)).unwrap()?;

        info!(
            net_loss = ?solution.net_loss,
            without_battery = ?net_loss_without_battery,
            profit = ?(net_loss_without_battery - solution.net_loss),
            charge = ?solution.charge,
            discharge = ?solution.discharge,
            elapsed = ?start_instant.elapsed(),
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
        interval: Interval,
        stand_by_power: Kilowatts,
        grid_rate: KilowattHourRate,
        initial_residual_energy: KilowattHours,
        min_residual_energy: KilowattHours,
        next_solutions: &[Option<Rc<Solution>>],
        max_energy: WattHours,
    ) -> Option<Solution> {
        let battery = Battery::builder()
            .residual_energy(initial_residual_energy)
            .min_residual_energy(min_residual_energy)
            .capacity(self.capacity)
            .parameters(self.battery_parameters)
            .build();
        self.working_modes
            .iter()
            .filter_map(|working_mode| {
                let step = self
                    .simulate_step()
                    .interval(interval)
                    .grid_rate(grid_rate)
                    .stand_by_power(stand_by_power)
                    .initial_residual_energy(initial_residual_energy)
                    .battery(battery)
                    .working_mode(working_mode)
                    .call();
                let next_solution = {
                    let next_energy = WattHours::from(step.residual_energy_after).min(max_energy);
                    next_solutions[usize::from(next_energy)].clone()
                }?;
                if step.residual_energy_after >= min_residual_energy {
                    Some(Solution {
                        net_loss: step.loss + next_solution.net_loss,
                        charge: step.charge() + next_solution.charge,
                        discharge: step.discharge() + next_solution.discharge,
                        payload: Some(Payload { step, next_solution }),
                    })
                } else {
                    // Do not allow dropping below the minimally allowed state-of-charge:
                    None
                }
            })
            .min_by_key(|partial_solution| partial_solution.net_loss)
    }

    /// Simulate the battery working in the specified mode given the initial conditions,
    /// and return the loss and new residual energy.
    #[builder]
    fn simulate_step(
        &self,
        mut battery: Battery,
        interval: Interval,
        stand_by_power: Kilowatts,
        grid_rate: KilowattHourRate,
        initial_residual_energy: KilowattHours,
        working_mode: WorkingMode,
    ) -> Step {
        let duration = interval.duration();

        // Requested external power flow to (positive) or from (negative) the battery:
        let battery_external_power = match working_mode {
            WorkingMode::Idle => Kilowatts::ZERO,
            WorkingMode::Backup => (-stand_by_power).max(Kilowatts::ZERO),
            WorkingMode::Charge => self.battery_args.charging_power,
            WorkingMode::Discharge => -self.battery_args.discharging_power,
            WorkingMode::Balance => (-stand_by_power)
                .clamp(-self.battery_args.discharging_power, self.battery_args.charging_power),
        };

        // Apply the load to the battery:
        let battery_active_time = battery.apply_load(battery_external_power, duration);

        // Total household energy balance:
        let grid_consumption =
            battery_external_power * battery_active_time + stand_by_power * duration;

        Step {
            interval,
            grid_rate,
            stand_by_power,
            working_mode,
            residual_energy_before: initial_residual_energy,
            residual_energy_after: battery.residual_energy(),
            grid_consumption,
            loss: self.loss(grid_rate, grid_consumption),
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

#[derive(Clone)]
pub struct Solution {
    /// Net loss from the current state till the forecast period end – our primary optimization target.
    net_loss: Cost,

    /// Cumulative charge.
    charge: KilowattHours,

    /// Cumulative discharge.
    discharge: KilowattHours,

    payload: Option<Payload>,
}

impl Solution {
    pub const fn new() -> Self {
        Self {
            net_loss: Cost::ZERO,
            charge: Quantity::ZERO,
            discharge: Quantity::ZERO,
            payload: None,
        }
    }

    /// Track the optimal solution till the end.
    pub fn backtrack(&self) -> impl Iterator<Item = Step> {
        let mut pointer = self;
        from_fn(move || {
            let current_payload = pointer.payload.as_ref()?;
            // …and advance:
            pointer = current_payload.next_solution.as_ref();
            Some(current_payload.step.clone())
        })
    }
}

/// Solution payload.
#[derive(Clone)]
pub struct Payload {
    /// The current (first step of the partial solution) step metrics.
    step: Step,

    /// Next partial solution – allows backtracking the entire sequence.
    ///
    /// I use [`Rc`] here to avoid storing the entire state matrix. That way, I calculate hour by
    /// hour, while moving from the future to the present. When all the states for the current hour
    /// are calculated, I can safely drop the previous hour states, because I keep the relevant
    /// links via [`Rc`].
    next_solution: Rc<Solution>,
}
