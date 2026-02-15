use tokio_modbus::SlaveId;
use url::Host;

/// Modbus slave connection endpoint.
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct Endpoint {
    pub host: Host,
    pub port: u16,
    pub slave_id: SlaveId,
}
