use std::{
    fmt::{Display, Formatter},
    iter::from_fn,
    rc::Rc,
};

use comfy_table::{Attribute, Cell, Color, Table, modifiers, presets};

use crate::{
    core::step::Step,
    quantity::{Quantity, cost::Cost, energy::KilowattHours},
};

#[derive(Clone)]
pub struct Solution {
    /// Net loss from the current state till the forecast period end – our primary optimization target.
    pub net_loss: Cost,

    /// Cumulative charge.
    pub charge: KilowattHours,

    /// Cumulative discharge.
    pub discharge: KilowattHours,

    pub payload: Option<Payload>,
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

    pub fn energy_flow(&self) -> KilowattHours {
        self.charge + self.discharge
    }

    pub const fn with_base_loss(&self, base_loss: Cost) -> SolutionSummary<'_> {
        SolutionSummary { base_loss, solution: self }
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
    /// The current step (first step of the partial solution) metrics.
    pub step: Step,

    /// Next partial solution – allows backtracking the entire sequence.
    ///
    /// I use [`Rc`] here to avoid storing the entire state matrix. That way, I calculate hour by
    /// hour, while moving from the future to the present. When all the states for the current hour
    /// are calculated, I can safely drop the previous hour states, because I keep the relevant
    /// links via [`Rc`].
    pub next_solution: Rc<Solution>,
}

#[must_use]
pub struct SolutionSummary<'a> {
    solution: &'a Solution,

    /// Estimated loss without using the battery.
    base_loss: Cost,
}

impl SolutionSummary<'_> {
    fn profit(&self) -> Cost {
        self.base_loss - self.solution.net_loss
    }
}

impl Display for SolutionSummary<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut table = Table::new();
        table
            .load_preset(presets::UTF8_FULL_CONDENSED)
            .apply_modifier(modifiers::UTF8_ROUND_CORNERS)
            .enforce_styling()
            .set_header(vec![
                Cell::from("Net profit"),
                Cell::from("Flow profit"),
                Cell::from("Charge"),
                Cell::from("Discharge"),
                Cell::from("Base loss"),
                Cell::from("Loss"),
            ])
            .add_row(vec![
                Cell::from(self.profit()),
                Cell::from(self.profit() / self.solution.energy_flow())
                    .add_attribute(Attribute::Bold),
                Cell::from(self.solution.charge).fg(Color::Green),
                Cell::from(self.solution.discharge).fg(Color::Red),
                Cell::from(self.base_loss),
                Cell::from(self.solution.net_loss),
            ]);
        write!(f, "{table}")
    }
}
