//! Implementation of the datat types specified here: https://supabase.com/docs/guides/realtime/protocol

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event")]
#[serde(rename_all = "snake_case")]
pub enum ProtocolMesseage {
    #[serde(rename = "phx_join")]
    PhxJoin(PhoenixMessage<phx_join::PhxJoin>),
    #[serde(rename = "phx_reply")]
    PhxReply(PhoenixMessage<phx_reply::PhxReply>),
    #[serde(rename = "presence_state")]
    PresenceState(PhoenixMessage<presence_state::PresenceState>),
    #[serde(rename = "system")]
    System(PhoenixMessage<system::System>),
    #[serde(rename = "phx_error")]
    PhxError(PhoenixMessage<phx_error::PhxError>),
    #[serde(rename = "postgres_changes")]
    PostgresChanges(PhoenixMessage<postgres_changes::PostgresChangesPayload>),

}

impl ProtocolMesseage {
    pub fn set_access_token(&mut self, new_access_token: &str) {
        match self {
            ProtocolMesseage::PhxJoin(PhoenixMessage {
                payload: phx_join::PhxJoin { access_token, .. },
                ..
            }) => {
                access_token.replace(new_access_token.to_owned());
            }
            ProtocolMesseage::PhxReply(_) => {
                // no op
            }
            ProtocolMesseage::PresenceState(_) => {}
            ProtocolMesseage::System(_) => {}
            ProtocolMesseage::PhxError(_) => {}
            ProtocolMesseage::PostgresChanges(_) => {}
        }
    }
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

pub mod phx_reply {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(tag = "status", content = "response")]
    #[serde(rename_all = "snake_case")]
    pub enum PhxReply {
        #[serde(rename = "error")]
        Error(ErrorReply),
        #[serde(rename = "ok")]
        Ok(PhxReplyQuery),
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct PhxReplyQuery {
        #[serde(rename = "postgres_changes")]
        pub postgres_changes: Vec<PostgresChanges>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct ErrorReply {
        reason: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct PostgresChanges {
        pub event: PostgresChangetEvent,
        pub schema: String,
        pub table: String,
        pub filter: Option<String>,
        pub id: i32,
    }
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub enum PostgresChangetEvent {
        #[serde(rename = "*")]
        All,
    }

    #[cfg(test)]
    mod tests {

        use super::*;

        #[test]
        fn test_error_serialisation() {
            let json_data = r#"
                {
                  "event": "phx_reply",
                  "payload": {
                    "response": {
                      "reason": "Invalid JWT Token"
                    },
                    "status": "error"
                  },
                  "ref": "1",
                  "topic": "realtime:db"
                }        "#;

            let expected_struct = ProtocolMesseage::PhxReply(PhoenixMessage {
                topic: "realtime:db".to_string(),
                payload: PhxReply::Error(ErrorReply {
                    reason: "Invalid JWT Token".to_string(),
                }),
                ref_field: Some("1".to_string()),
                join_ref: None,
            });
            let serialzed = serde_json::to_value(&expected_struct).unwrap();
            dbg!(serialzed);

            let deserialized_struct: ProtocolMesseage = serde_json::from_str(json_data).unwrap();

            assert_eq!(deserialized_struct, expected_struct);
        }

        #[test]
        fn test_filter_query_event_serialisation() {
            let json_data = r#"
            {
            "event": "phx_reply",
            "payload": {
            "response": {
              "postgres_changes": [
                {
                  "event": "*",
                  "filter": "id=eq.83a19c16-fcd8-45d0-9710-d7b06ce6f329",
                  "id": 31339675,
                  "schema": "public",
                  "table": "profiles"
                }
              ]
            },
            "status": "ok"
          },
          "ref": "1",
          "topic": "realtime:db"
        } "#;

            let expected_struct: ProtocolMesseage = ProtocolMesseage::PhxReply(PhoenixMessage {
                topic: "realtime:db".to_string(),
                payload: PhxReply::Ok(PhxReplyQuery {
                    postgres_changes: vec![PostgresChanges {
                        schema: "public".to_string(),
                        table: "profiles".to_string(),
                        id: 31339675,
                        filter: Some("id=eq.83a19c16-fcd8-45d0-9710-d7b06ce6f329".to_string()),
                        event: PostgresChangetEvent::All,
                    }],
                }),
                ref_field: Some("1".to_string()),
                join_ref: None,
            });

            let serialzed = serde_json::to_value(&expected_struct).unwrap();
            dbg!(serialzed);

            let deserialized_struct: ProtocolMesseage = serde_json::from_str(json_data).unwrap();

            assert_eq!(deserialized_struct, expected_struct);
        }

        #[test]
        fn test_event_query_serialisation() {
            let json_data = r#"
            {
            "event": "phx_reply",
            "payload": {
            "response": {
                "postgres_changes": [
            {
            "event": "*",
            "filter": "",
            "id": 30636876,
            "schema": "public",
            "table": "profiles"
            }
            ]
        },
        "status": "ok"
        },
            "ref": "1",
            "topic":  "realtime:db"
            } "#;

            let expected_struct: ProtocolMesseage = ProtocolMesseage::PhxReply(PhoenixMessage {
                topic: "realtime:db".to_string(),
                payload: PhxReply::Ok(PhxReplyQuery {
                    postgres_changes: vec![PostgresChanges {
                        schema: "public".to_string(),
                        table: "profiles".to_string(),
                        id: 30636876,
                        filter: Some("".to_string()),
                        event: PostgresChangetEvent::All,
                    }],
                }),
                ref_field: Some("1".to_string()),
                join_ref: None,
            });

            let serialzed = serde_json::to_value(&expected_struct).unwrap();
            dbg!(serialzed);

            let deserialized_struct: ProtocolMesseage = serde_json::from_str(json_data).unwrap();

            assert_eq!(deserialized_struct, expected_struct);
        }
    }
}

pub mod phx_join {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct PhxJoin {
        #[serde(rename = "config")]
        pub config: JoinConfig,
        #[serde(rename = "access_token")]
        pub access_token: Option<String>,
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
        pub filter: Option<String>,
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

            let expected_struct = ProtocolMesseage::PhxJoin(PhoenixMessage {
                topic: "realtime:db".to_string(),
                payload: phx_join::PhxJoin {
                    config: phx_join::JoinConfig {
                    broadcast: phx_join::BroadcastConfig {
                        self_item: false,
                        ack: false
                    },
                    presence: phx_join::PresenceConfig {
                        key: "".to_string()
                    },
                    postgres_changes: vec![
                            PostgrsChanges {
                                event: PostgresChangetEvent::All,
                                schema: "public".to_string(),
                                table: "profiles".to_string(),
                                filter: Some("id=eq.83a19c16-fcd8-45d0-9710-d7b06ce6f329".to_string()),
                            }
                        ],
                    },
                    access_token: Some("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJhYWwiOiJhYWwxIiwiYW1yIjpbeyJtZXRob2QiOiJwYXNzd29yZCIsInRpbWVzdGFtcCI6MTcyMDU0NzU4Nn1dLCJhcHBfbWV0YWRhdGEiOnsicHJvdmlkZXIiOiJlbWFpbCIsInByb3ZpZGVycyI6WyJlbWFpbCJdfSwiYXVkIjoiYXV0aGVudGljYXRlZCIsImVtYWlsIjoic2NvdXRAc3dvb3BzY29yZS5jb20iLCJleHAiOjE3MjA2MzQ3NjMsImlhdCI6MTcyMDYzNDcwMywiaXNfYW5vbnltb3VzIjpmYWxzZSwiaXNzIjoiaHR0cDovLzEyNy4wLjAuMTo1NDMyMS9hdXRoL3YxIiwicGhvbmUiOiIiLCJyb2xlIjoiYXV0aGVudGljYXRlZCIsInNlc3Npb25faWQiOiJiMGQ5ODY4Mi0zYTEwLTQxOTAtYWZjZC01NWE5Nzc2MGEzZTYiLCJzdWIiOiI4M2ExOWMxNi1mY2Q4LTQ1ZDAtOTcxMC1kN2IwNmNlNmYzMjkiLCJ1c2VyX21ldGFkYXRhIjp7fSwidXNlcl9yb2xlIjoic2NvdXQifQ.Smmu7aH808WzYT3Z82pQGxZQ2NmDsKZL64rG1uJ_wtQ".to_string()),
                },
                ref_field: Some("1".to_string()),
                join_ref: Some("1".to_string()),
            });
            let serialzed = serde_json::to_value(&expected_struct).unwrap();
            dbg!(serialzed);

            let deserialized_struct: ProtocolMesseage = serde_json::from_str(json_data).unwrap();

            assert_eq!(deserialized_struct, expected_struct);
        }
    }
}

pub mod presence_state {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub struct PresenceState;

    #[cfg(test)]
    mod tests {
        use super::*;
      
        #[test]
        fn test_presence_state_serialization() {
            let json_data = r#"
            {
                "event": "presence_state",
                "payload": {},
                "ref": null,
                "topic": "realtime:db"
            }
            "#;

            let expected_struct = ProtocolMesseage::PresenceState(PhoenixMessage {
                topic: "realtime:db".to_string(),
                payload:  PresenceState {},
                ref_field: None,
                join_ref: None,
            });

            let serialzed = serde_json::to_value(&expected_struct).unwrap();
            dbg!(serialzed);

            let deserialized_struct: ProtocolMesseage = serde_json::from_str(json_data).unwrap();

            assert_eq!(deserialized_struct, expected_struct);
        }
    }
}




pub mod system {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    
    pub struct System {
        #[serde(rename = "channel")]
        pub channel: String,
        #[serde(rename = "extension")]
        pub extension: String,
        #[serde(rename = "message")]
        pub message: String,
        #[serde(rename = "status")]
        pub status: String,
    }


    #[cfg(test)]
    mod tests {
        use super::*;
      
        #[test]
        fn test_system_subscribe_error_serialization() {
            let json_data = r#"
            {
            "event": "system",
            "payload": {
            "channel": "db",
            "extension": "postgres_changes",
            "message": "{:error, \"Unable to subscribe to changes with given parameters. Please check Realtime is enabled for the given connect parameters: [event: *, filter: id=eq.83a19c16-fcd8-45d0-9710-d7b06ce6f329, schema: public, table: profiles]\"}",
            "status": "error"
                },
            "ref": null,
            "topic": "realtime:db"
            }
            "#;

            let expected_struct = ProtocolMesseage::System(PhoenixMessage {
                topic: "realtime:db".to_string(),
                payload: system::System {
                    channel: "db".to_string(),
                    extension: "postgres_changes".to_string(),
                    message:"{:error, \"Unable to subscribe to changes with given parameters. Please check Realtime is enabled for the given connect parameters: [event: *, filter: id=eq.83a19c16-fcd8-45d0-9710-d7b06ce6f329, schema: public, table: profiles]\"}".to_string(),
                    status: "error".to_string(),
                },
                ref_field: None,
                join_ref: None,
            });

            dbg!(&expected_struct);

            let serialzed = serde_json::to_value(&expected_struct).unwrap();
            dbg!(serialzed);

            let deserialized_struct: ProtocolMesseage = serde_json::from_str(json_data).unwrap();

            assert_eq!(deserialized_struct, expected_struct);
        }

        #[test]
        fn test_system_error_parse_filter_serialization() {
            let json_data = r#"
            {
                "event": "system",
                "payload": {
                "channel": "db",
                "extension": "postgres_changes",
                "message": "{:error, \"Error parsing `filter` params: [\\\"\\\"]\"}",
                "status": "error"
                },
                "ref": null,
                "topic": "realtime:db"
            }
            "#;

            let expected_struct = ProtocolMesseage::System(PhoenixMessage {
                topic: "realtime:db".to_string(),
                payload: system::System {
                    channel: "db".to_string(),
                    extension: "postgres_changes".to_string(),
                    message:"{:error, \"Error parsing `filter` params: [\\\"\\\"]\"}".to_string(),
                    status: "error".to_string(),
                },
                ref_field: None,
                join_ref: None,
            });

            dbg!(&expected_struct);

            let serialzed = serde_json::to_value(&expected_struct).unwrap();
            dbg!(serialzed);

            let deserialized_struct: ProtocolMesseage = serde_json::from_str(json_data).unwrap();

            assert_eq!(deserialized_struct, expected_struct);
        }

        #[test]
        fn test_system_ok_psql_sub_serialization() {
            let json_data = r#"
            {
                "event": "system",
                "payload": {
                "channel": "db",
                "extension": "postgres_changes",
                "message": "Subscribed to PostgreSQL",
                "status": "ok"
                },
                "ref": null,
                "topic": "realtime:db"
            }
            "#;

            let expected_struct = ProtocolMesseage::System(PhoenixMessage {
                topic: "realtime:db".to_string(),
                payload: system::System {
                    channel: "db".to_string(),
                    extension: "postgres_changes".to_string(),
                    message:"Subscribed to PostgreSQL".to_string(),
                    status: "ok".to_string(),
                },
                ref_field: None,
                join_ref: None,
            });

            dbg!(&expected_struct);

            let serialzed = serde_json::to_value(&expected_struct).unwrap();
            dbg!(serialzed);

            let deserialized_struct: ProtocolMesseage = serde_json::from_str(json_data).unwrap();

            assert_eq!(deserialized_struct, expected_struct);
        }
    }
}


pub mod phx_error {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub struct PhxError;

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_phx_error_serialization() {
            let json_data = r#"
            {
                "event": "phx_error",
                "payload": {},
                "ref": "1",
                "topic": "realtime:db"
            }
            "#;

            let expected_struct = ProtocolMesseage::PhxError(PhoenixMessage {
                topic: "realtime:db".to_string(),
                payload: PhxError {},
                ref_field: Some("1".to_string()),
                join_ref: None,
            });

            let serialized = serde_json::to_value(&expected_struct).unwrap();
            dbg!(serialized);

            let deserialized_struct: ProtocolMesseage = serde_json::from_str(json_data).unwrap();
            dbg!(&deserialized_struct);

            assert_eq!(deserialized_struct, expected_struct);
        }
    }
}


pub mod postgres_changes {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub struct PostgresChangesPayload {
        pub data: Data,
        pub ids: Vec<i64>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub struct Data {
        pub columns: Vec<Column>,
        #[serde(rename = "commit_timestamp")]
        pub commit_timestamp: String,
        pub errors: Option<String>,
        #[serde(rename = "old_record")]
        pub old_record: Option<Record>,
        pub record: Record,
        pub schema: String,
        pub table: String,
        #[serde(rename = "type")]
        pub type_: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub struct Column {
        pub name: String,
        #[serde(rename = "type")]
        pub type_: String, 
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub struct Record {
        #[serde(rename = "id")]
        pub id: String,
        #[serde(rename = "updated_at")]
        pub updated_at: Option<String>, 
        #[serde(rename = "url")]
        pub url: Option<String>, 
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_postgres_changes_serialization() {
            let json_data = r#"
            {
                "event": "postgres_changes",
                "payload": {
                    "data": {
                        "columns": [
                            {"name": "id", "type": "uuid"},
                            {"name": "updated_at", "type": "timestamptz"},
                            {"name": "url", "type": "text"}
                        ],
                        "commit_timestamp": "2024-08-25T17:00:19.009Z",
                        "errors": null,
                        "old_record": {
                            "id": "96236356-5ac3-4403-b3ce-c660973330d9"
                        },
                        "record": {
                            "id": "96236356-5ac3-4403-b3ce-c660973330d9",
                            "updated_at": "2024-08-25T17:00:19.005328+00:00",
                            "url": "https://0.0.0.0:3334"
                        },
                        "schema": "public",
                        "table": "profiles",
                        "type": "UPDATE"
                    },
                    "ids": [38606455]
                },
                "ref": null,
                "topic": "realtime:db"
            }
            "#;

            let expected_struct = ProtocolMesseage::PostgresChanges(PhoenixMessage {
                topic: "realtime:db".to_string(),
                payload: PostgresChangesPayload {
                    data: Data {
                        columns: vec![
                            Column { name: "id".to_string(), type_: "uuid".to_string() },
                            Column { name: "updated_at".to_string(), type_: "timestamptz".to_string() },
                            Column { name: "url".to_string(), type_: "text".to_string() },
                        ],
                        commit_timestamp: "2024-08-25T17:00:19.009Z".to_string(),
                        errors: None,
                        old_record: Some(Record {
                            id: "96236356-5ac3-4403-b3ce-c660973330d9".to_string(),
                            updated_at: None,
                            url: None,
                        }),
                        record: Record {
                            id: "96236356-5ac3-4403-b3ce-c660973330d9".to_string(),
                            updated_at: Some("2024-08-25T17:00:19.005328+00:00".to_string()),
                            url: Some("https://0.0.0.0:3334".to_string()),
                        },
                        schema: "public".to_string(),
                        table: "profiles".to_string(),
                        type_: "UPDATE".to_string(),
                    },
                    ids: vec![38606455],
                },
                ref_field: None,
                join_ref: None,
            });

            let serialized = serde_json::to_value(&expected_struct).unwrap();
            dbg!(serialized);

            let deserialized_struct: ProtocolMesseage = serde_json::from_str(json_data).unwrap();
            dbg!(&deserialized_struct);

            assert_eq!(deserialized_struct, expected_struct);
        }
    }
}


