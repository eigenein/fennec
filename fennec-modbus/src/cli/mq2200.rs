use fennec_modbus::{
    contrib::mq2200,
    protocol::address,
    tcp::{UnitId, tokio::Client},
};

pub async fn read(client: Client<String>, unit_id: UnitId) -> anyhow::Result<()> {
    println!(
        "State-of-health: {:?}",
        client.call::<mq2200::ReadStateOfHealth>(unit_id, address::Const).await?
    );
    println!(
        "Design capacity: {:?}",
        client.call::<mq2200::ReadDesignCapacity>(unit_id, address::Const).await?
    );
    println!(
        "Total active power: {:?}",
        client.call::<mq2200::ReadTotalActivePower>(unit_id, address::Const).await?
    );
    println!(
        "Total EPS active power: {:?}",
        client.call::<mq2200::ReadEpsActivePower>(unit_id, address::Const).await?
    );
    println!(
        "State-of-charge: {:?}",
        client.call::<mq2200::ReadStateOfCharge>(unit_id, address::Const).await?
    );
    println!(
        "Minimum system SoC: {:?}",
        client.call::<mq2200::ReadMinimumSystemStateOfCharge>(unit_id, address::Const).await?
    );
    println!(
        "Maximum SoC: {:?}",
        client.call::<mq2200::ReadMaximumStateOfCharge>(unit_id, address::Const).await?
    );
    println!(
        "Minimum SoC on grid: {:?}",
        client.call::<mq2200::ReadMinimumStateOfChargeOnGrid>(unit_id, address::Const).await?
    );
    for i in 0..mq2200::schedule::BlockIndex::MAX {
        let schedule_block = client
            .call::<mq2200::ReadScheduleEntryBlock>(unit_id, mq2200::schedule::BlockIndex(i))
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
                entry.minimum_state_of_charge.0,
                entry.maximum_state_of_charge.0
            );
        }
    }
    Ok(())
}
