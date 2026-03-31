use maud::{Markup, Render, html};

use crate::quantity::energy::WattHours;

pub struct ResidualEnergyIconText {
    pub residual_energy: WattHours,
    pub actual_capacity: WattHours,
}

impl Render for ResidualEnergyIconText {
    fn render(&self) -> Markup {
        let state_of_charge = self.residual_energy / self.actual_capacity;
        let class = if state_of_charge >= 0.8 {
            "fa-battery-full"
        } else if state_of_charge >= 0.6 {
            "fa-battery-three-quarters"
        } else if state_of_charge >= 0.4 {
            "fa-battery-half"
        } else if state_of_charge >= 0.2 {
            "fa-battery-quarter"
        } else {
            "fa-battery-empty"
        };
        html! {
            span.icon-text.is-flex-wrap-nowrap {
                span.icon { i.fas.(class) {} }
                span { (self.residual_energy) }
            }
        }
    }
}
