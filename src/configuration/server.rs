use serde::Deserialize;

use crate::configuration::listeners::{ListenerBaseConfig, ServerTlsConfig, TlsListenerConfig};

/// Which listener the `[server]` section asks for.
///
/// Deserialized through [`ServerConfigFields`] rather than as an untagged enum:
/// untagged tries each variant in turn and takes the first that parses, so a
/// `[server.tls]` section with a missing or invalid key fell through to the
/// plaintext variant and silently downgraded the listener.
#[derive(Clone, Debug, Deserialize)]
#[serde(from = "ServerConfigFields")]
pub enum ServerConfig {
    Tls(TlsListenerConfig),
    Insecure(ListenerBaseConfig),
}

#[derive(Deserialize)]
struct ServerConfigFields {
    #[serde(flatten)]
    base: ListenerBaseConfig,
    tls: Option<ServerTlsConfig>,
}

impl From<ServerConfigFields> for ServerConfig {
    fn from(fields: ServerConfigFields) -> Self {
        match fields.tls {
            Some(tls) => Self::Tls(TlsListenerConfig {
                base: fields.base,
                tls,
            }),
            None => Self::Insecure(fields.base),
        }
    }
}
