use fennec_modbus::{
    contrib::mini_qube::{functions, schedule},
    protocol::address,
    tcp::{UnitId, tokio::Client},
};

pub async fn read(client: Client<String>, unit_id: UnitId) -> anyhow::Result<()> {
    println!(
        "State-of-health: {:?}",
        client.call::<functions::ReadStateOfHealth>(unit_id, address::Const).await?
    );
    println!(
        "Design capacity: {:?}",
        client.call::<functions::ReadDesignCapacity>(unit_id, address::Const).await?
    );
    println!(
        "Total active power: {:?}",
        client.call::<functions::ReadTotalActivePower>(unit_id, address::Const).await?
    );
    println!(
        "Total EPS active power: {:?}",
        client.call::<functions::ReadEpsActivePower>(unit_id, address::Const).await?
    );
    println!(
        "State-of-charge: {:?}",
        client.call::<functions::ReadStateOfCharge>(unit_id, address::Const).await?
    );
    println!(
        "Minimum system SoC: {:?}",
        client.call::<functions::ReadMinimumSystemStateOfCharge>(unit_id, address::Const).await?
    );
    println!(
        "Maximum SoC: {:?}",
        client.call::<functions::ReadMaximumStateOfCharge>(unit_id, address::Const).await?
    );
    println!(
        "Minimum SoC on grid: {:?}",
        client.call::<functions::ReadMinimumStateOfChargeOnGrid>(unit_id, address::Const).await?
    );
    for i in 0..=schedule::BlockIndex::LAST {
        let schedule_block = client
            .call::<functions::ReadScheduleEntryBlock>(unit_id, schedule::BlockIndex(i))
            .await?;
        for entry in schedule_block {
            println!(
                "{} - {}: enabled={} mode={:?} target_soc={} watts={} soc_range={}..={}",
                entry.start_time,
                entry.end_time,
                entry.is_enabled,
                entry.working_mode,
                entry.target_state_of_charge.0,
                entry.power.0,
                entry.min_state_of_charge.0,
                entry.max_state_of_charge.0
            );
        }
    }
    Ok(())
}
