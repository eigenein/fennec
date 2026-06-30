use fennec_modbus::{
    contrib::mini_qube,
    protocol::address,
    tcp::{UnitId, tokio::Client},
};

pub async fn read(client: Client<String>, unit_id: UnitId) -> anyhow::Result<()> {
    println!(
        "State-of-health: {:?}",
        client.call::<mini_qube::ReadStateOfHealth>(unit_id, address::Const).await?
    );
    println!(
        "Design capacity: {:?}",
        client.call::<mini_qube::ReadDesignCapacity>(unit_id, address::Const).await?
    );
    println!(
        "Total active power: {:?}",
        client.call::<mini_qube::ReadTotalActivePower>(unit_id, address::Const).await?
    );
    println!(
        "Total EPS active power: {:?}",
        client.call::<mini_qube::ReadEpsActivePower>(unit_id, address::Const).await?
    );
    println!(
        "State-of-charge: {:?}",
        client.call::<mini_qube::ReadStateOfCharge>(unit_id, address::Const).await?
    );
    println!(
        "Minimum system SoC: {:?}",
        client.call::<mini_qube::ReadMinimumSystemStateOfCharge>(unit_id, address::Const).await?
    );
    println!(
        "Maximum SoC: {:?}",
        client.call::<mini_qube::ReadMaximumStateOfCharge>(unit_id, address::Const).await?
    );
    println!(
        "Minimum SoC on grid: {:?}",
        client.call::<mini_qube::ReadMinimumStateOfChargeOnGrid>(unit_id, address::Const).await?
    );
    for i in 0..mini_qube::schedule::BlockIndex::MAX {
        let schedule_block = client
            .call::<mini_qube::ReadScheduleEntryBlock>(unit_id, mini_qube::schedule::BlockIndex(i))
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
