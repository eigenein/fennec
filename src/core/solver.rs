mod energy;
pub mod solution;
pub mod step;
pub mod summary;

use bon::{Builder, bon, builder};
use chrono::{DateTime, Local, Timelike};
use ordered_float::OrderedFloat;

use crate::{
    cli::{BatteryArgs, ConsumptionArgs},
    core::{
        metrics::Metrics,
        series::Series,
        solver::{energy::DecawattHours, solution::Solution, step::Step, summary::Summary},
        working_mode::WorkingMode,
    },
    prelude::*,
    units::{
        currency::Cost,
        energy::KilowattHours,
        power::Kilowatts,
        quantity::Quantity,
        rate::KilowattHourRate,
        surface_area::SquareMetres,
        time::Hours,
    },
};

#[derive(Builder)]
#[builder(finish_fn(vis = ""))]
pub struct Solver<'a> {
    metrics: &'a Series<Metrics>,
    pv_surface_area: SquareMetres,
    residual_energy: KilowattHours,
    capacity: KilowattHours,
    battery: BatteryArgs,
    consumption: ConsumptionArgs,
    stand_by_power: [Option<Kilowatts>; 24],
}

impl<S: solver_builder::IsComplete> SolverBuilder<'_, S> {
    pub fn solve(self) -> Solution {
        self.build().solve()
    }
}

#[bon]
impl Solver<'_> {
    /// Find the optimal battery schedule.
    #[instrument(skip_all, name = "Solving…", fields(residual_energy = %self.residual_energy))]
    fn solve(self) -> Solution {
        let min_residual_energy = self.capacity * f64::from(self.battery.min_soc_percent) / 100.0;
        let max_energy = DecawattHours::from(self.residual_energy.max(self.capacity));
        let n_energy_states = usize::from(max_energy) + 1;

        let mut net_loss_without_battery = Cost::ZERO;
        let mut next_hour_losses = vec![Cost::ZERO; n_energy_states];
        let mut backtracks = Vec::with_capacity(self.metrics.len());

        for (timestamp, metrics) in self.metrics.into_iter().rev() {
            let stand_by_power =
                self.stand_by_power[timestamp.hour() as usize].unwrap_or(self.consumption.stand_by);

            // For missing weather forecast, assume none solar power:
            let solar_production =
                metrics.solar_power_density.unwrap_or(Quantity::ZERO) * self.pv_surface_area;

            // Positive is excess, negative is deficit:
            let power_balance = solar_production - stand_by_power;

            // Subtracting because we benefit from positive power balance:
            net_loss_without_battery -= self.loss(metrics.grid_rate, power_balance * Hours::ONE);

            let mut net_losses = Vec::with_capacity(n_energy_states);
            let mut linked_steps = Vec::with_capacity(n_energy_states);

            for decawatt_hours in 0..=max_energy.0 {
                let initial_residual_energy = KilowattHours::from(DecawattHours(decawatt_hours));
                let partial_solution = self
                    .optimise_hour()
                    .stand_by_power(stand_by_power)
                    .power_balance(power_balance)
                    .grid_rate(metrics.grid_rate)
                    .initial_residual_energy(initial_residual_energy)
                    .min_residual_energy(min_residual_energy)
                    .next_hour_losses(&next_hour_losses)
                    .max_energy(max_energy)
                    .call();
                net_losses.push(partial_solution.net_loss);
                // FIXME: introduce some kind of `LinkedStep` type:
                linked_steps.push((partial_solution.next_energy, partial_solution.step));
            }
            next_hour_losses = net_losses;
            // FIXME: introduce some kind of `Backtrack` type:
            backtracks.push((*timestamp, linked_steps));
        }

        // By this moment, «next hour losses» is actually the upcoming hour, so our solution is:
        let initial_energy = DecawattHours::from(self.residual_energy);
        let net_loss = next_hour_losses[usize::from(initial_energy)];

        Solution {
            summary: Summary { net_loss, net_loss_without_battery },
            steps: Self::backtrack(initial_energy, backtracks),
        }
    }

    #[builder]
    fn optimise_hour(
        &self,
        stand_by_power: Kilowatts,
        power_balance: Kilowatts,
        grid_rate: KilowattHourRate,
        initial_residual_energy: KilowattHours,
        min_residual_energy: KilowattHours,
        next_hour_losses: &[Cost],
        max_energy: DecawattHours,
    ) -> PartialSolution {
        [WorkingMode::Idle, WorkingMode::Discharging, WorkingMode::Balancing, WorkingMode::Charging]
            .into_iter()
            .map(|working_mode| {
                let step = self
                    .simulate_hour()
                    .stand_by_power(stand_by_power)
                    .grid_rate(grid_rate)
                    .initial_residual_energy(initial_residual_energy)
                    .min_residual_energy(min_residual_energy)
                    .working_mode(working_mode)
                    .power_balance(power_balance)
                    .call();

                let next_energy = DecawattHours::from(step.residual_energy_after).min(max_energy);
                let net_loss = step.loss + next_hour_losses[usize::from(next_energy)];
                PartialSolution { net_loss, next_energy, step: Some(step) }
            })
            .min_by_key(|partial_solution| OrderedFloat(partial_solution.net_loss.0))
            .unwrap()
    }

    /// Simulate the battery working in the specified mode given the initial conditions,
    /// and return the loss and new residual energy.
    #[builder]
    fn simulate_hour(
        &self,
        stand_by_power: Kilowatts,
        grid_rate: KilowattHourRate,
        initial_residual_energy: KilowattHours,
        min_residual_energy: KilowattHours,
        working_mode: WorkingMode,
        power_balance: Kilowatts,
    ) -> Step {
        let mut current_residual_energy = initial_residual_energy;

        // Power flow to the battery (negative is directed from the battery):
        let battery_external_power = match working_mode {
            WorkingMode::Idle => Kilowatts::ZERO,
            WorkingMode::Charging => self.battery.charging_power,
            WorkingMode::Discharging => -self.battery.discharging_power,
            WorkingMode::Balancing => {
                power_balance.clamp(-self.battery.discharging_power, self.battery.charging_power)
            }
        };

        // Power flow inside the battery corrected by the round-trip efficiency:
        let (battery_external_power, battery_active_time) =
            if battery_external_power > Kilowatts::ZERO {
                // While charging, the residual energy grows slower:
                let internal_power = battery_external_power * self.battery.efficiency;
                current_residual_energy = (current_residual_energy + internal_power * Hours::ONE)
                    .min(self.capacity.max(initial_residual_energy));
                let time_charging =
                    (current_residual_energy - initial_residual_energy) / internal_power;
                assert!(time_charging >= Hours::ZERO);
                (battery_external_power, time_charging)
            } else if battery_external_power < Kilowatts::ZERO {
                // While discharging, the residual energy is spent faster:
                let internal_power = battery_external_power / self.battery.efficiency;
                // Remember that the power here is negative, hence the `+`:
                current_residual_energy = (current_residual_energy + internal_power * Hours::ONE)
                    .max(min_residual_energy.min(initial_residual_energy));
                let time_discharging =
                    (current_residual_energy - initial_residual_energy) / internal_power;
                assert!(time_discharging >= Hours::ZERO);
                (battery_external_power, time_discharging)
            } else {
                // Idle:
                (Kilowatts::ZERO, Hours::ZERO)
            };

        // Self-discharging:
        current_residual_energy = (current_residual_energy
            - self.battery.self_discharge * (Hours::ONE - battery_active_time))
            .max(KilowattHours::ZERO);

        // Finally, total household energy balance:
        let production_without_battery = power_balance * Hours::ONE;
        let grid_consumption =
            battery_external_power * battery_active_time - production_without_battery;

        Step {
            working_mode,
            residual_energy_before: initial_residual_energy,
            residual_energy_after: current_residual_energy,
            stand_by_power,
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
            consumption * (grid_rate - self.consumption.purchase_fees)
        }
    }

    #[expect(clippy::type_complexity)] // FIXME
    fn backtrack(
        initial_energy: DecawattHours,
        backtracks: Vec<(DateTime<Local>, Vec<(DecawattHours, Option<Step>)>)>,
    ) -> Series<Step> {
        let mut energy = initial_energy;
        backtracks
            .into_iter()
            .rev()
            .map(|(timestamp, linked_steps)| {
                let (next_energy, step) = linked_steps[usize::from(energy)];
                energy = next_energy;
                (timestamp, step.unwrap())
            })
            .collect()
    }
}

#[derive(Copy, Clone)]
struct PartialSolution {
    net_loss: Cost,
    next_energy: DecawattHours,
    step: Option<Step>,
}
