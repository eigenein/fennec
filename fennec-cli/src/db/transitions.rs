use turso::Connection;

use crate::{db::transition::ResidualEnergyTransition, prelude::*};

pub struct Transitions<'c>(pub &'c Connection);

impl Transitions<'_> {
    #[instrument(skip_all, fields(energy = ?transition.energy))]
    pub async fn upsert(&self, transition: &ResidualEnergyTransition) -> Result {
        // language=sqlite
        const SQL: &str = r"
            INSERT INTO residual_energy_transitions (timestamp_millis, milliwatt_hours) VALUES (?1, ?2)
            ON CONFLICT DO UPDATE SET milliwatt_hours = ?2
        ";

        info!("upserting the transition…");
        self.0
            .prepare_cached(SQL)
            .await?
            .execute((transition.timestamp.timestamp_millis(), transition.energy))
            .await?;
        Ok(())
    }
}
