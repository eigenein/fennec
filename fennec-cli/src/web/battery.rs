use maud::{Markup, html};

use crate::quantity::energy::WattHours;

pub struct StateOfCharge {
    pub residual_energy: WattHours,
    pub actual_capacity: WattHours,
}

impl StateOfCharge {
    pub fn icon(&self) -> Markup {
        let class = {
            let state_of_charge = self.residual_energy / self.actual_capacity;
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
        };
        html! {
            span.icon { i.fas.(class) {} }
        }
    }

    pub fn class(&self) -> &'static str {
        let state_of_charge = self.residual_energy / self.actual_capacity;
        if state_of_charge >= 0.75 {
            "is-success"
        } else if state_of_charge >= 0.5 {
            "is-info"
        } else if state_of_charge >= 0.25 {
            "is-warning"
        } else {
            "is-danger"
        }
    }
}
