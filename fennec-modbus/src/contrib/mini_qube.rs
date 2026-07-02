//! Calls for [Fox ESS MQ2200 (MiniQube)][1], [Solakon ONE][2], and Avocado 22 Pro.
//!
//! [1]: https://fox-ess.uk/miniqube/
//! [2]: https://www.solakon.de/products/solakon-one

use crate::tcp;

pub mod functions;
pub mod schedule;
pub mod types;

/// Default unit ID ("slave ID") for MiniQube over direct TCP connection.
pub const UNIT_ID: tcp::UnitId = tcp::UnitId::Significant(1);
