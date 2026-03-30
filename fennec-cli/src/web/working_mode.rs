use maud::Render;

use crate::battery::WorkingMode;

pub struct WorkingModeColor(pub WorkingMode);

impl Render for WorkingModeColor {
    fn render_to(&self, buffer: &mut String) {
        match self.0 {
            WorkingMode::Idle => {}
            WorkingMode::Harness => buffer.push_str("is-primary"),
            WorkingMode::Compensate => buffer.push_str("is-danger"),
            WorkingMode::SelfUse => buffer.push_str("is-warning"),
            WorkingMode::Charge => buffer.push_str("is-success"),
            WorkingMode::Discharge => buffer.push_str("is-info"),
        }
    }
}
