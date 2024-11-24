//! Implementation of the datat types specified here: <https://supabase.com/docs/guides/realtime/protocol>

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtocolMessage {
    pub topic: String,
    #[serde(flatten)]
    pub payload: ProtocolPayload,
    #[serde(rename = "ref")]
    pub ref_field: Option<String>,
    pub join_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", content = "payload", rename_all = "snake_case")]
pub enum ProtocolPayload {
    #[serde(rename = "heartbeat")]
    Heartbeat(heartbeat::Heartbeat),
    #[serde(rename = "access_token")]
    AccessToken(access_token::AccessToken),
    #[serde(rename = "phx_join")]
    PhxJoin(phx_join::PhxJoin),
    #[serde(rename = "phx_close")]
    PhxClose(phx_close::PhxClose),
    #[serde(rename = "phx_reply")]
    PhxReply(phx_reply::PhxReply),
    #[serde(rename = "presence_state")]
    PresenceState(presence_state::PresenceState),
    #[serde(rename = "broadcast")]
    Broadcast(broadcast::Broadcast),
    #[serde(rename = "presence_diff")]
    PresenceDiff(presence_diff::PresenceDiff),
    #[serde(rename = "system")]
    System(system::System),
    #[serde(rename = "phx_error")]
    PhxError(phx_error::PhxError),
    #[serde(rename = "postgres_changes")]
    PostgresChanges(postgres_changes::PostgresChangesPayload),
}

impl ProtocolMessage {
    pub fn set_access_token(&mut self, new_access_token: &str) {
        match &mut self.payload {
            ProtocolPayload::PhxJoin(phx_join::PhxJoin { access_token, .. }) => {
                access_token.replace(new_access_token.to_owned());
            }
            ProtocolPayload::AccessToken(access_token::AccessToken { access_token }) => {
                *access_token = new_access_token.to_owned();
            }
            _ => {}
        }
    }
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
        #[serde(default)]
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
        use pretty_assertions::assert_eq;

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

            let expected_struct = ProtocolMessage {
                topic: "realtime:db".to_owned(),
                payload: ProtocolPayload::PhxReply(PhxReply::Error(ErrorReply {
                    reason: "Invalid JWT Token".to_owned(),
                })),
                ref_field: Some("1".to_owned()),
                join_ref: None,
            };
            let serialzed = simd_json::to_string_pretty(&expected_struct).unwrap();
            dbg!(serialzed);

            let deserialized_struct: ProtocolMessage =
                simd_json::from_slice(json_data.to_owned().into_bytes().as_mut_slice()).unwrap();

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

            let expected_struct = ProtocolMessage {
                topic: "realtime:db".to_owned(),
                payload: ProtocolPayload::PhxReply(PhxReply::Ok(PhxReplyQuery {
                    postgres_changes: vec![PostgresChanges {
                        schema: "public".to_owned(),
                        table: "profiles".to_owned(),
                        id: 31339675,
                        filter: Some("id=eq.83a19c16-fcd8-45d0-9710-d7b06ce6f329".to_owned()),
                        event: PostgresChangetEvent::All,
                    }],
                })),
                ref_field: Some("1".to_owned()),
                join_ref: None,
            };

            let serialzed = simd_json::to_string_pretty(&expected_struct).unwrap();
            dbg!(serialzed);

            let deserialized_struct: ProtocolMessage =
                simd_json::from_slice(json_data.to_owned().into_bytes().as_mut_slice()).unwrap();

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

            let expected_struct = ProtocolMessage {
                topic: "realtime:db".to_owned(),
                payload: ProtocolPayload::PhxReply(PhxReply::Ok(PhxReplyQuery {
                    postgres_changes: vec![PostgresChanges {
                        schema: "public".to_owned(),
                        table: "profiles".to_owned(),
                        id: 30636876,
                        filter: Some(String::new()),
                        event: PostgresChangetEvent::All,
                    }],
                })),
                ref_field: Some("1".to_owned()),
                join_ref: None,
            };

            let serialzed = simd_json::to_string_pretty(&expected_struct).unwrap();
            dbg!(serialzed);
            let deserialized_struct: ProtocolMessage =
                simd_json::from_slice(json_data.to_owned().into_bytes().as_mut_slice()).unwrap();

            assert_eq!(deserialized_struct, expected_struct);
        }
    }

    #[test]
    fn test_ok_empty_response_serialisation() {
        let json_data = r#"
    {
        "ref": null,
        "event": "phx_reply",
        "payload": {
            "status": "ok",
            "response": {}
        },
        "topic": "phoenix"
    }"#;

        let expected_struct = ProtocolMessage {
            topic: "phoenix".to_owned(),
            payload: ProtocolPayload::PhxReply(PhxReply::Ok(PhxReplyQuery {
                postgres_changes: Vec::new(),
            })),
            ref_field: None,
            join_ref: None,
        };

        let serialized = simd_json::to_string_pretty(&expected_struct).unwrap();
        dbg!(serialized);

        let deserialized_struct: ProtocolMessage =
            simd_json::from_slice(json_data.to_owned().into_bytes().as_mut_slice()).unwrap();

        assert_eq!(deserialized_struct, expected_struct);
    }
}

pub mod phx_join {
    use super::*;

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
                "access_token": "your_access_token"
            },
            "ref": "1",
            "join_ref": "1"
        }
        "#;

            let expected_struct = ProtocolMessage {
                topic: "realtime:db".to_owned(),
                payload: ProtocolPayload::PhxJoin(PhxJoin {
                    config: JoinConfig {
                        broadcast: BroadcastConfig {
                            self_item: false,
                            ack: false,
                        },
                        presence: PresenceConfig { key: String::new() },
                        postgres_changes: vec![PostgrsChanges {
                            event: PostgresChangetEvent::All,
                            schema: "public".to_owned(),
                            table: "profiles".to_owned(),
                            filter: Some("id=eq.83a19c16-fcd8-45d0-9710-d7b06ce6f329".to_owned()),
                        }],
                    },
                    access_token: Some("your_access_token".to_owned()),
                }),
                ref_field: Some("1".to_owned()),
                join_ref: Some("1".to_owned()),
            };
            let serialzed = simd_json::to_string_pretty(&expected_struct).unwrap();
            dbg!(serialzed);

            let deserialized_struct: ProtocolMessage =
                simd_json::from_slice(json_data.to_owned().into_bytes().as_mut_slice()).unwrap();

            assert_eq!(deserialized_struct, expected_struct);
        }
    }
}

pub mod presence_state {
    use std::collections::HashMap;

    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct PresenceState(pub HashMap<String, Presence>);

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct Presence {
        pub metas: Vec<PresenceMeta>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct PresenceMeta {
        pub phx_ref: String,
        pub name: String,
        pub t: f64,
    }

    #[cfg(test)]
    mod tests {
        use pretty_assertions::assert_eq;

        use super::*;
        use crate::message::{ProtocolMessage, ProtocolPayload};

        #[test]
        fn test_presence_state_deserialization() {
            let json_data = r#"
            {
                "ref": null,
                "event": "presence_state",
                "payload": {
                    "1c4ed5ca-aaa4-11ef-bce9-0242ac120004": {
                        "metas": [
                            {
                                "phx_ref": "GAsCC3FpEhdb4wgk",
                                "name": "service_role_75",
                                "t": 22866011
                            }
                        ]
                    }
                },
                "topic": "realtime:af"
            }
            "#;

            let mut state_map = HashMap::new();
            state_map.insert(
                "1c4ed5ca-aaa4-11ef-bce9-0242ac120004".to_owned(),
                Presence {
                    metas: vec![PresenceMeta {
                        phx_ref: "GAsCC3FpEhdb4wgk".to_owned(),
                        name: "service_role_75".to_owned(),
                        t: 22866011.0,
                    }],
                },
            );

            let expected_struct = ProtocolMessage {
                topic: "realtime:af".to_owned(),
                payload: ProtocolPayload::PresenceState(PresenceState(state_map)),
                ref_field: None,
                join_ref: None,
            };

            let serialzed = simd_json::to_string_pretty(&expected_struct).unwrap();
            dbg!(serialzed);

            let deserialized_struct: ProtocolMessage =
                simd_json::from_slice(json_data.to_owned().into_bytes().as_mut_slice()).unwrap();

            assert_eq!(deserialized_struct, expected_struct);
        }
    }
}

pub mod broadcast {
    use simd_json::OwnedValue;

    use super::*;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct Broadcast {
        #[serde(rename = "type")]
        pub r#type: String,
        pub event: String,
        pub payload: OwnedValue,
    }

    #[cfg(test)]
    mod tests {
        use pretty_assertions::assert_eq;
        use simd_json::json;

        use super::*;

        #[test]
        fn test_broadcast_deserialization() {
            let json_data = r#"{
                "ref": null,
                "event": "broadcast",
                "payload": {
                    "event": "Test message",
                    "payload": {
                        "message": "Hello World"
                    },
                    "type": "broadcast"
                },
                "topic": "realtime:af"
            }"#;

            let expected_struct = ProtocolMessage {
                topic: "realtime:af".to_owned(),
                payload: ProtocolPayload::Broadcast(Broadcast {
                    r#type: "broadcast".to_owned(),
                    event: "Test message".to_owned(),
                    payload: json!({
                        "message": "Hello World"
                    }),
                }),
                ref_field: None,
                join_ref: None,
            };

            let serialzed = simd_json::to_string_pretty(&expected_struct).unwrap();
            dbg!(serialzed);

            let deserialized_struct: ProtocolMessage =
                simd_json::from_slice(json_data.to_owned().into_bytes().as_mut_slice()).unwrap();

            assert_eq!(deserialized_struct, expected_struct);
        }

        #[test]
        fn test_broadcast_deserialization_second_example() {
            let json_data = r#"{
                "topic": "realtime:af",
                "event": "broadcast",
                "payload": {
                    "type": "broadcast",
                    "event": "message",
                    "payload": {
                        "content": "dddd"
                    }
                },
                "ref": "3",
                "join_ref": "1"
            }"#;

            let expected_struct = ProtocolMessage {
                topic: "realtime:af".to_owned(),
                payload: ProtocolPayload::Broadcast(Broadcast {
                    r#type: "broadcast".to_owned(),
                    event: "message".to_owned(),
                    payload: json!({
                        "content": "dddd"
                    }),
                }),
                ref_field: Some("3".to_owned()),
                join_ref: Some("1".to_owned()),
            };

            let serialzed = simd_json::to_string_pretty(&expected_struct).unwrap();
            dbg!(serialzed);

            let deserialized_struct: ProtocolMessage =
                simd_json::from_slice(json_data.to_owned().into_bytes().as_mut_slice()).unwrap();

            assert_eq!(deserialized_struct, expected_struct);
        }
    }
}
pub mod presence_diff {
    use std::collections::HashMap;

    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct PresenceDiff {
        pub joins: HashMap<String, Presence>,
        pub leaves: HashMap<String, Presence>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct Presence {
        pub metas: Vec<PresenceMeta>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct PresenceMeta {
        pub phx_ref: String,
        pub name: String,
        pub t: f64,
    }

    #[cfg(test)]
    mod tests {
        use pretty_assertions::assert_eq;

        use super::*;
        use crate::message::{ProtocolMessage, ProtocolPayload};

        #[test]
        fn test_presence_diff_deserialization() {
            let json_data = r#"
            {
                "ref": null,
                "event": "presence_diff",
                "payload": {
                    "joins": {
                        "fe9f9386-aaa1-11ef-a588-0242ac120004": {
                            "metas": [
                                {
                                    "phx_ref": "GAsBN9izrRlb40jh",
                                    "name": "service_role_47",
                                    "t": 21957173.599999905
                                }
                            ]
                        }
                    },
                    "leaves": {}
                },
                "topic": "realtime:af"
            }
            "#;

            let expected_struct = ProtocolMessage {
                topic: "realtime:af".to_owned(),
                payload: ProtocolPayload::PresenceDiff(PresenceDiff {
                    joins: {
                        let mut joins = HashMap::new();
                        joins.insert(
                            "fe9f9386-aaa1-11ef-a588-0242ac120004".to_owned(),
                            Presence {
                                metas: vec![PresenceMeta {
                                    phx_ref: "GAsBN9izrRlb40jh".to_owned(),
                                    name: "service_role_47".to_owned(),
                                    t: 21957173.599999905,
                                }],
                            },
                        );
                        joins
                    },
                    leaves: HashMap::new(),
                }),
                ref_field: None,
                join_ref: None,
            };

            let serialzed = simd_json::to_string_pretty(&expected_struct).unwrap();
            dbg!(serialzed);

            let deserialized_struct: ProtocolMessage =
                simd_json::from_slice(json_data.to_owned().into_bytes().as_mut_slice()).unwrap();

            assert_eq!(deserialized_struct, expected_struct);
        }
    }
}

pub mod heartbeat {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub struct Heartbeat;

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_presence_state_serialization() {
            let json_data = r#"
            {
               "event": "heartbeat",
               "topic": "phoenix",
               "payload": {},
               "ref": null
            }
            "#;

            let expected_struct = ProtocolMessage {
                topic: "phoenix".to_owned(),
                payload: ProtocolPayload::Heartbeat(Heartbeat),
                ref_field: None,
                join_ref: None,
            };

            let serialzed = simd_json::to_string_pretty(&expected_struct).unwrap();
            dbg!(serialzed);

            let deserialized_struct: ProtocolMessage =
                simd_json::from_slice(json_data.to_owned().into_bytes().as_mut_slice()).unwrap();

            assert_eq!(deserialized_struct, expected_struct);
        }
    }
}

pub mod access_token {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub struct AccessToken {
        pub access_token: String,
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_access_token() {
            let json_data = r#"
            {
               "event": "access_token",
               "topic": "realtime::something::something",
               "payload":{
                  "access_token": "ssss"
               },
               "ref": null
            }
            "#;

            let expected_struct = ProtocolMessage {
                topic: "realtime::something::something".to_owned(),
                payload: ProtocolPayload::AccessToken(AccessToken {
                    access_token: "ssss".to_owned(),
                }),
                ref_field: None,
                join_ref: None,
            };

            let serialzed = simd_json::to_string_pretty(&expected_struct).unwrap();
            dbg!(serialzed);

            let deserialized_struct: ProtocolMessage =
                simd_json::from_slice(json_data.to_owned().into_bytes().as_mut_slice()).unwrap();

            assert_eq!(deserialized_struct, expected_struct);
        }
    }
}

pub mod phx_close {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub struct PhxClose {}

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_phx_close() {
            let json_data = r#"
            {
               "event": "phx_close",
               "topic": "realtime::something::something",
               "payload":{},
               "ref": null
            }
            "#;

            let expected_struct = ProtocolMessage {
                topic: "realtime::something::something".to_owned(),
                payload: ProtocolPayload::PhxClose(PhxClose {}),
                ref_field: None,
                join_ref: None,
            };

            let serialzed = simd_json::to_string_pretty(&expected_struct).unwrap();
            dbg!(serialzed);

            let deserialized_struct: ProtocolMessage =
                simd_json::from_slice(json_data.to_owned().into_bytes().as_mut_slice()).unwrap();

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

            let expected_struct = ProtocolMessage {
                topic: "realtime:db".to_owned(),
                payload: ProtocolPayload::System(System {
                    channel: "db".to_owned(),
                    extension: "postgres_changes".to_owned(),
                    message: "{:error, \"Unable to subscribe to changes with given parameters. Please check Realtime is enabled for the given connect parameters: [event: *, filter: id=eq.83a19c16-fcd8-45d0-9710-d7b06ce6f329, schema: public, table: profiles]\"}".to_owned(),
                    status: "error".to_owned(),
                }),
                ref_field: None,
                join_ref: None,
            };

            dbg!(&expected_struct);

            let serialzed = simd_json::to_string_pretty(&expected_struct).unwrap();
            dbg!(serialzed);

            let deserialized_struct: ProtocolMessage =
                simd_json::from_slice(json_data.to_owned().into_bytes().as_mut_slice()).unwrap();

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

            let expected_struct = ProtocolMessage {
                topic: "realtime:db".to_owned(),
                payload: ProtocolPayload::System(System {
                    channel: "db".to_owned(),
                    extension: "postgres_changes".to_owned(),
                    message: "{:error, \"Error parsing `filter` params: [\\\"\\\"]\"}".to_owned(),
                    status: "error".to_owned(),
                }),
                ref_field: None,
                join_ref: None,
            };

            dbg!(&expected_struct);

            let serialzed = simd_json::to_string_pretty(&expected_struct).unwrap();
            dbg!(serialzed);

            let deserialized_struct: ProtocolMessage =
                simd_json::from_slice(json_data.to_owned().into_bytes().as_mut_slice()).unwrap();

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

            let expected_struct = ProtocolMessage {
                topic: "realtime:db".to_owned(),
                payload: ProtocolPayload::System(System {
                    channel: "db".to_owned(),
                    extension: "postgres_changes".to_owned(),
                    message: "Subscribed to PostgreSQL".to_owned(),
                    status: "ok".to_owned(),
                }),
                ref_field: None,
                join_ref: None,
            };

            dbg!(&expected_struct);

            let serialzed = simd_json::to_string_pretty(&expected_struct).unwrap();
            dbg!(serialzed);

            let deserialized_struct: ProtocolMessage =
                simd_json::from_slice(json_data.to_owned().into_bytes().as_mut_slice()).unwrap();

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

            let expected_struct = ProtocolMessage {
                topic: "realtime:db".to_owned(),
                payload: ProtocolPayload::PhxError(PhxError),
                ref_field: Some("1".to_owned()),
                join_ref: None,
            };

            let serialized = simd_json::to_string_pretty(&expected_struct).unwrap();
            dbg!(serialized);

            let deserialized_struct: ProtocolMessage =
                simd_json::from_slice(json_data.to_owned().into_bytes().as_mut_slice()).unwrap();
            dbg!(&deserialized_struct);

            assert_eq!(deserialized_struct, expected_struct);
        }
    }
}

pub mod postgres_changes {
    use alloc::fmt;

    use serde::de::{self, DeserializeOwned};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub struct PostgresChangesPayload {
        pub data: Data<Buffer, Buffer>,
        pub ids: Vec<i64>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub struct Column {
        pub name: String,
        #[serde(rename = "type")]
        pub type_: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub enum PostgresDataChangeEvent {
        #[serde(rename = "INSERT")]
        Insert,
        #[serde(rename = "UPDATE")]
        Update,
        #[serde(rename = "DELETE")]
        Delete,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub struct Data<R = Buffer, O = Buffer> {
        pub columns: Vec<Column>,
        #[serde(rename = "commit_timestamp")]
        pub commit_timestamp: String,
        #[serde(default)]
        pub errors: Option<String>,
        #[serde(default, rename = "old_record")]
        pub old_record: Option<O>,
        #[serde(default)]
        pub record: Option<R>,
        pub schema: String,
        pub table: String,
        #[serde(rename = "type")]
        pub type_: PostgresDataChangeEvent,
    }

    impl<O> Data<Buffer, O> {
        /// Parses the `record` field and returns a new `Data` instance with the parsed type.
        pub fn parse_record<T: DeserializeOwned>(self) -> Result<Data<T, O>, simd_json::Error> {
            let record = match self.record {
                Some(buffer) => {
                    let mut data = buffer.into_inner();
                    let parsed: T = simd_json::from_slice(&mut data)?;
                    Some(parsed)
                }
                None => None,
            };

            Ok(Data {
                record,
                old_record: self.old_record,
                columns: self.columns,
                commit_timestamp: self.commit_timestamp,
                errors: self.errors,
                schema: self.schema,
                table: self.table,
                type_: self.type_,
            })
        }
    }
    impl<R> Data<R, Buffer> {
        /// Parses the `old_record` field and returns a new `Data` instance with the parsed type.
        pub fn parse_old_record<K: DeserializeOwned>(self) -> Result<Data<R, K>, simd_json::Error> {
            let old_record = match self.old_record {
                Some(buffer) => {
                    let mut data = buffer.into_inner();
                    let parsed: K = simd_json::from_slice(&mut data)?;
                    Some(parsed)
                }
                None => None,
            };

            Ok(Data {
                record: self.record,
                old_record,
                columns: self.columns,
                commit_timestamp: self.commit_timestamp,
                errors: self.errors,
                schema: self.schema,
                table: self.table,
                type_: self.type_,
            })
        }
    }

    #[derive(Debug, Default, Clone, PartialEq, Eq)]
    pub struct Buffer(pub Vec<u8>);

    impl Buffer {
        #[must_use]
        pub fn into_inner(self) -> Vec<u8> {
            self.0
        }
    }

    // Implement Serialize for Buffer
    impl Serialize for Buffer {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let mut bytes_copy = self.0.clone();
            let json_value: simd_json::OwnedValue =
                simd_json::to_owned_value(&mut bytes_copy).map_err(serde::ser::Error::custom)?;
            json_value.serialize(serializer)
        }
    }

    // Implement Deserialize for Buffer
    impl<'de> Deserialize<'de> for Buffer {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct RawBytesVisitor;

            impl<'de> de::Visitor<'de> for RawBytesVisitor {
                type Value = Buffer;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("a JSON value")
                }

                fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
                where
                    D: Deserializer<'de>,
                {
                    let value = simd_json::BorrowedValue::deserialize(deserializer)?;
                    let buf = simd_json::to_vec(&value).map_err(de::Error::custom)?;
                    Ok(Buffer(buf))
                }

                fn visit_none<E>(self) -> Result<Self::Value, E>
                where
                    E: de::Error,
                {
                    Ok(Buffer(Vec::new()))
                }

                fn visit_unit<E>(self) -> Result<Self::Value, E>
                where
                    E: de::Error,
                {
                    self.visit_none()
                }

                fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
                where
                    A: de::SeqAccess<'de>,
                {
                    let value = serde::de::value::SeqAccessDeserializer::new(seq);
                    let value = simd_json::BorrowedValue::deserialize(value)?;
                    let buf = simd_json::to_vec(&value).map_err(de::Error::custom)?;
                    Ok(Buffer(buf))
                }

                fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
                where
                    A: de::MapAccess<'de>,
                {
                    let value = serde::de::value::MapAccessDeserializer::new(map);
                    let value = simd_json::BorrowedValue::deserialize(value)?;
                    let buf = simd_json::to_vec(&value).map_err(de::Error::custom)?;
                    Ok(Buffer(buf))
                }
            }

            deserializer.deserialize_any(RawBytesVisitor)
        }
    }

    #[cfg(test)]
    mod tests {
        use pretty_assertions::assert_eq;

        use super::*;
        use crate::message::{ProtocolMessage, ProtocolPayload};

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

            // Parse json_data to extract raw bytes for record and old_record using simd_json
            let mut json_data_bytes = json_data.to_owned().into_bytes();
            let json_value: simd_json::OwnedValue =
                simd_json::from_slice(&mut json_data_bytes).unwrap();
            let data = &json_value["payload"]["data"];
            let record_value = &data["record"];
            let old_record_value = &data["old_record"];

            // Serialize record_value and old_record_value to bytes
            let record_bytes = simd_json::to_vec(record_value).unwrap();
            let old_record_bytes = simd_json::to_vec(old_record_value).unwrap();

            let expected_struct = ProtocolMessage {
                topic: "realtime:db".to_owned(),
                payload: ProtocolPayload::PostgresChanges(PostgresChangesPayload {
                    data: Data {
                        columns: vec![
                            Column {
                                name: "id".to_owned(),
                                type_: "uuid".to_owned(),
                            },
                            Column {
                                name: "updated_at".to_owned(),
                                type_: "timestamptz".to_owned(),
                            },
                            Column {
                                name: "url".to_owned(),
                                type_: "text".to_owned(),
                            },
                        ],
                        commit_timestamp: "2024-08-25T17:00:19.009Z".to_owned(),
                        errors: None,
                        old_record: Some(Buffer(old_record_bytes)),
                        record: Some(Buffer(record_bytes)),
                        schema: "public".to_owned(),
                        table: "profiles".to_owned(),
                        type_: PostgresDataChangeEvent::Update,
                    },
                    ids: vec![38606455],
                }),
                ref_field: None,
                join_ref: None,
            };

            // Deserialize json_data using simd_json
            let mut deserialized_bytes = json_data.to_owned().into_bytes();
            let deserialized_struct: ProtocolMessage =
                simd_json::from_slice(&mut deserialized_bytes).unwrap();

            // Assert equality
            assert_eq!(deserialized_struct, expected_struct);
        }

        #[test]
        fn complex_data_insert() {
            let json_data = r#"
         {
             "ref": null,
             "event": "postgres_changes",
             "payload": {
                 "data": {
                     "table": "rooms",
                     "type": "INSERT",
                     "record": {
                         "created_at": "2024-10-19T07:55:12.92041+00:00",
                         "id": "cb099344-62b7-4ee0-a3ab-ec178486b685",
                         "name": "dddddddd",
                         "owner_id": "c791e9bf-4d77-4ac9-adb7-d351927c4416",
                         "server_id": "eb7619f1-be42-4904-bee1-01e0bb11e795"
                     },
                     "columns": [
                         {
                             "name": "id",
                             "type": "uuid"
                         },
                         {
                             "name": "created_at",
                             "type": "timestamptz"
                         },
                         {
                             "name": "name",
                             "type": "text"
                         },
                         {
                             "name": "owner_id",
                             "type": "uuid"
                         },
                         {
                             "name": "server_id",
                             "type": "uuid"
                         }
                     ],
                     "errors": null,
                     "commit_timestamp": "2024-10-19T07:55:12.926Z",
                     "schema": "public"
                 },
                 "ids": [
                     60402389
                 ]
             },
             "topic": "realtime:table-db-changes"
         }
         "#;

            // Parse json_data to extract raw bytes for record and old_record using simd_json
            let mut json_data_bytes = json_data.to_owned().into_bytes();
            let json_value: simd_json::OwnedValue =
                simd_json::from_slice(&mut json_data_bytes).unwrap();
            let data = &json_value["payload"]["data"];
            let record_value = &data["record"];

            // Serialize record_value and old_record_value to bytes
            let record_bytes = simd_json::to_vec(record_value).unwrap();

            let expected_struct = ProtocolMessage {
                topic: "realtime:table-db-changes".to_owned(),
                payload: ProtocolPayload::PostgresChanges(PostgresChangesPayload {
                    data: Data {
                        table: "rooms".to_owned(),
                        type_: PostgresDataChangeEvent::Insert,
                        record: Some(Buffer(record_bytes)),
                        old_record: None,
                        columns: vec![
                            Column {
                                name: "id".to_owned(),
                                type_: "uuid".to_owned(),
                            },
                            Column {
                                name: "created_at".to_owned(),
                                type_: "timestamptz".to_owned(),
                            },
                            Column {
                                name: "name".to_owned(),
                                type_: "text".to_owned(),
                            },
                            Column {
                                name: "owner_id".to_owned(),
                                type_: "uuid".to_owned(),
                            },
                            Column {
                                name: "server_id".to_owned(),
                                type_: "uuid".to_owned(),
                            },
                        ],
                        commit_timestamp: "2024-10-19T07:55:12.926Z".to_owned(),
                        errors: None,
                        schema: "public".to_owned(),
                    },
                    ids: vec![60402389],
                }),
                ref_field: None,
                join_ref: None,
            };

            let serialized = simd_json::to_string_pretty(&expected_struct).unwrap();
            dbg!(serialized);

            let deserialized_struct: ProtocolMessage =
                simd_json::from_slice(json_data.to_owned().into_bytes().as_mut_slice()).unwrap();
            dbg!(&deserialized_struct);

            pretty_assertions::assert_eq!(deserialized_struct, expected_struct);
        }

        #[test]
        fn complex_data_delete() {
            let json_data = r#"
         {
             "ref": null,
             "event": "postgres_changes",
             "payload": {
                 "data": {
                     "table": "rooms",
                     "type": "DELETE",
                     "columns": [
                         {
                             "name": "id",
                             "type": "uuid"
                         },
                         {
                             "name": "created_at",
                             "type": "timestamptz"
                         },
                         {
                             "name": "name",
                             "type": "text"
                         },
                         {
                             "name": "owner_id",
                             "type": "uuid"
                         },
                         {
                             "name": "server_id",
                             "type": "uuid"
                         }
                     ],
                     "errors": null,
                     "commit_timestamp": "2024-10-19T07:54:05.101Z",
                     "schema": "public",
                     "old_record": {
                         "id": "c722fce5-a3cf-4af2-b261-609612b884c1"
                     }
                 },
                 "ids": [
                     38377940
                 ]
             },
             "topic": "realtime:table-db-changes"
         }
         "#;

            // Parse json_data to extract raw bytes for record and old_record using simd_json
            let mut json_data_bytes = json_data.to_owned().into_bytes();
            let json_value: simd_json::OwnedValue =
                simd_json::from_slice(&mut json_data_bytes).unwrap();
            let data = &json_value["payload"]["data"];
            let old_record_value = &data["old_record"];

            // Serialize record_value and old_record_value to bytes
            let old_record_bytes = simd_json::to_vec(old_record_value).unwrap();

            let expected_struct = ProtocolMessage {
                topic: "realtime:table-db-changes".to_owned(),
                payload: ProtocolPayload::PostgresChanges(PostgresChangesPayload {
                    data: Data {
                        table: "rooms".to_owned(),
                        type_: PostgresDataChangeEvent::Delete,
                        record: None,
                        old_record: Some(Buffer(old_record_bytes)),
                        columns: vec![
                            Column {
                                name: "id".to_owned(),
                                type_: "uuid".to_owned(),
                            },
                            Column {
                                name: "created_at".to_owned(),
                                type_: "timestamptz".to_owned(),
                            },
                            Column {
                                name: "name".to_owned(),
                                type_: "text".to_owned(),
                            },
                            Column {
                                name: "owner_id".to_owned(),
                                type_: "uuid".to_owned(),
                            },
                            Column {
                                name: "server_id".to_owned(),
                                type_: "uuid".to_owned(),
                            },
                        ],
                        commit_timestamp: "2024-10-19T07:54:05.101Z".to_owned(),
                        errors: None,
                        schema: "public".to_owned(),
                    },
                    ids: vec![38377940],
                }),
                ref_field: None,
                join_ref: None,
            };

            let serialized = simd_json::to_string_pretty(&expected_struct).unwrap();
            dbg!(serialized);

            let deserialized_struct: ProtocolMessage =
                simd_json::from_slice(json_data.to_owned().into_bytes().as_mut_slice()).unwrap();
            dbg!(&deserialized_struct);

            assert_eq!(deserialized_struct, expected_struct);
        }
    }
}
