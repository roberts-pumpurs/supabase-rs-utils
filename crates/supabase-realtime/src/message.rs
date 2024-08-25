use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event")]
#[serde(rename_all = "snake_case")]
pub enum InboundProtocolMesseage {
    #[serde(rename = "phx_join")]
    PhxJoin(PhoenixMessage<PhxJoin>),
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
pub struct PhxJoin {
    #[serde(rename = "config")]
    pub config: JoinConfig,
    #[serde(rename = "access_token")]
    pub access_token: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JoinConfig {
    #[serde(rename = "broadcast")]
    pub broadcast: BroadcastConfig,
    #[serde(rename = "presence")]
    pub presence: PresenceConfig,
    #[serde(rename = "postgres_changes")]
    pub postgres_changes: Vec<PostgrsChanges>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BroadcastConfig {
    #[serde(rename = "self")]
    pub self_item: bool,
    #[serde(rename = "ack")]
    pub ack: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresenceConfig {
    pub key: String,
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

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_json_serialization() {
        let json_data = r#"
        {
            "topic": "realtime:db",
            "event": "phx_join",
            "payload": {
                "config": {
                    "broadcast": {
                        "ack": false,
                        "self": false
                    },
                    "presence": {
                        "key": ""
                    },
                    "postgres_changes": [
                        {
                            "event": "*",
                            "schema": "public",
                            "table": "profiles",
                            "filter": "id=eq.83a19c16-fcd8-45d0-9710-d7b06ce6f329"
                        }
                    ]
                },
                "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJhYWwiOiJhYWwxIiwiYW1yIjpbeyJtZXRob2QiOiJwYXNzd29yZCIsInRpbWVzdGFtcCI6MTcyMDU0NzU4Nn1dLCJhcHBfbWV0YWRhdGEiOnsicHJvdmlkZXIiOiJlbWFpbCIsInByb3ZpZGVycyI6WyJlbWFpbCJdfSwiYXVkIjoiYXV0aGVudGljYXRlZCIsImVtYWlsIjoic2NvdXRAc3dvb3BzY29yZS5jb20iLCJleHAiOjE3MjA2MzQ3NjMsImlhdCI6MTcyMDYzNDcwMywiaXNfYW5vbnltb3VzIjpmYWxzZSwiaXNzIjoiaHR0cDovLzEyNy4wLjAuMTo1NDMyMS9hdXRoL3YxIiwicGhvbmUiOiIiLCJyb2xlIjoiYXV0aGVudGljYXRlZCIsInNlc3Npb25faWQiOiJiMGQ5ODY4Mi0zYTEwLTQxOTAtYWZjZC01NWE5Nzc2MGEzZTYiLCJzdWIiOiI4M2ExOWMxNi1mY2Q4LTQ1ZDAtOTcxMC1kN2IwNmNlNmYzMjkiLCJ1c2VyX21ldGFkYXRhIjp7fSwidXNlcl9yb2xlIjoic2NvdXQifQ.Smmu7aH808WzYT3Z82pQGxZQ2NmDsKZL64rG1uJ_wtQ"
            },
            "ref": "1",
            "join_ref": "1"
        }
        "#;

        let expected_struct = InboundProtocolMesseage::PhxJoin(PhoenixMessage {
            topic: "realtime:db".to_string(),
            payload: PhxJoin {
                config: JoinConfig {
                broadcast: BroadcastConfig {
                    self_item: false,
                    ack: false
                },
                presence: PresenceConfig {
                    key: "".to_string()
                },
                postgres_changes: vec![
                    PostgrsChanges {
                        event: PostgresChangetEvent::All,
                        schema: "public".to_string(),
                        table: "profiles".to_string(),
                        filter: "id=eq.83a19c16-fcd8-45d0-9710-d7b06ce6f329".to_string(),
                    }
                ],
            },
                access_token: "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJhYWwiOiJhYWwxIiwiYW1yIjpbeyJtZXRob2QiOiJwYXNzd29yZCIsInRpbWVzdGFtcCI6MTcyMDU0NzU4Nn1dLCJhcHBfbWV0YWRhdGEiOnsicHJvdmlkZXIiOiJlbWFpbCIsInByb3ZpZGVycyI6WyJlbWFpbCJdfSwiYXVkIjoiYXV0aGVudGljYXRlZCIsImVtYWlsIjoic2NvdXRAc3dvb3BzY29yZS5jb20iLCJleHAiOjE3MjA2MzQ3NjMsImlhdCI6MTcyMDYzNDcwMywiaXNfYW5vbnltb3VzIjpmYWxzZSwiaXNzIjoiaHR0cDovLzEyNy4wLjAuMTo1NDMyMS9hdXRoL3YxIiwicGhvbmUiOiIiLCJyb2xlIjoiYXV0aGVudGljYXRlZCIsInNlc3Npb25faWQiOiJiMGQ5ODY4Mi0zYTEwLTQxOTAtYWZjZC01NWE5Nzc2MGEzZTYiLCJzdWIiOiI4M2ExOWMxNi1mY2Q4LTQ1ZDAtOTcxMC1kN2IwNmNlNmYzMjkiLCJ1c2VyX21ldGFkYXRhIjp7fSwidXNlcl9yb2xlIjoic2NvdXQifQ.Smmu7aH808WzYT3Z82pQGxZQ2NmDsKZL64rG1uJ_wtQ".to_string(),
            } ,
            ref_field: Some("1".to_string()),
            join_ref: Some("1".to_string()),
        });
        let serialzed = serde_json::to_value(&expected_struct).unwrap();
        dbg!(serialzed);

        let deserialized_struct: InboundProtocolMesseage = serde_json::from_str(json_data).unwrap();

        assert_eq!(deserialized_struct, expected_struct);
    }
}
