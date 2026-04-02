use maud::{Markup, html};

use crate::quantity::energy::WattHours;

pub struct StateOfCharge {
    pub residual_energy: WattHours,
    pub actual_capacity: Option<WattHours>,
}

impl StateOfCharge {
    pub fn icon(&self) -> Markup {
        #[expect(clippy::option_if_let_else)]
        let class = if let Some(actual_capacity) = self.actual_capacity {
            let state_of_charge = self.residual_energy / actual_capacity;
            if state_of_charge >= 0.8 {
                "fa-battery-full"
            } else if state_of_charge >= 0.6 {
                "fa-battery-three-quarters"
            } else if state_of_charge >= 0.4 {
                "fa-battery-half"
            } else if state_of_charge >= 0.2 {
                "fa-battery-quarter"
            } else {
                "fa-battery-empty"
            }
        } else {
            "fa-battery-half"
        };
        html! {
            span.icon { i.fas.(class) {} }
        }
    }

    pub fn class(&self) -> &'static str {
        #[expect(clippy::option_if_let_else)]
        if let Some(actual_capacity) = self.actual_capacity {
            let state_of_charge = self.residual_energy / actual_capacity;
            if state_of_charge >= 0.75 {
                "is-success"
            } else if state_of_charge >= 0.5 {
                "is-info"
            } else if state_of_charge >= 0.25 {
                "is-warning"
            } else {
                "is-danger"
            }
        } else {
            ""
        }
    }
}
