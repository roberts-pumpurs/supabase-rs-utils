use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event")]
#[serde(rename_all = "snake_case")]
pub enum InboundProtocolMesseage {
    #[serde(rename = "phx_join")]
    PhxJoin(PhoenixMessage<JoinConfig>),
}

// Main struct generic over the event type and payload
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhoenixMessage<T> {
    pub topic: String,
    pub payload: T,
    #[serde(rename = "ref")]
    pub ref_field: Option<String>,
    pub join_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JoinConfig {
    #[serde(rename = "broadcast")]
    BroadcastConfig(BroadcastConfig),
    #[serde(rename = "presence")]
    PresenceConfig(PresenceConfig),
    #[serde(rename = "postgres_changes")]
    PostgresConfig(PostgresConfig),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BroadcastConfig {
    #[serde(rename = "self")]
    pub self_item: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresenceConfig {
    pub key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostgresConfig {
    #[serde(flatten)]
    pub items: Vec<PostgrsChanges>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostgrsChanges {
    pub event: PostgresChangetEvent,
    pub schema: String,
    pub table: String,
    pub filter: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PostgresChangetEvent {
    #[serde(rename = "*")]
    All,
    #[serde(rename = "INSERT")]
    Insert,
    #[serde(rename = "UPDATE")]
    Update,
    #[serde(rename = "DELETE")]
    Delete,
}
