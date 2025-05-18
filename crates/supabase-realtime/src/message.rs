//! Implementation of the datat types specified here: <https://supabase.com/docs/guides/realtime/protocol>

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Buffer(pub Vec<u8>);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtocolMessage {
    pub topic: String,
    #[serde(flatten)]
    pub payload: ProtocolPayload,
    #[serde(rename = "ref")]
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ref_field: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
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

    #[serde(rename = "broadcast")]
    Broadcast(broadcast::Broadcast),

    // presence
    #[serde(rename = "presence")]
    PresenceInner(presence_inner::PresenceInner),
    #[serde(rename = "presence_state")]
    PresenceState(presence_state::PresenceState),
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
                new_access_token.clone_into(access_token);
            }
            ProtocolPayload::Heartbeat(_)
            | ProtocolPayload::PhxClose(_)
            | ProtocolPayload::PhxReply(_)
            | ProtocolPayload::Broadcast(_)
            | ProtocolPayload::PresenceInner(_)
            | ProtocolPayload::PresenceState(_)
            | ProtocolPayload::PresenceDiff(_)
            | ProtocolPayload::System(_)
            | ProtocolPayload::PhxError(_)
            | ProtocolPayload::PostgresChanges(_) => {}
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
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        pub filter: Option<String>,
        pub id: i32,
    }
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub enum PostgresChangetEvent {
        #[serde(rename = "*")]
        All,
    }

    #[cfg(test)]
    #[expect(clippy::unwrap_used, reason = "Allowed in test code for simplicity")]
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
                        id: 31_339_675,
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
                        id: 30_636_876,
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

    #[cfg(test)]
    #[expect(clippy::unwrap_used, reason = "Allowed in test code for simplicity")]
    mod test_ok_empty_response_serialisation_mod {
        use super::*;
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
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
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
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        pub filter: Option<String>,
    }

    #[cfg(test)]
    #[expect(clippy::unwrap_used, reason = "Allowed in test code for simplicity")]
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
        pub name: Option<String>,
        #[serde(flatten)]
        pub payload: simd_json::OwnedValue,
    }

    #[cfg(test)]
    #[expect(clippy::unwrap_used, reason = "Allowed in test code for simplicity")]
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
                        name: Some("service_role_75".to_owned()),
                        payload: simd_json::json!({"t": 22_866_011_u64 }),
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

pub mod presence_inner {

    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct PresenceInner {
        #[serde(rename = "type")]
        pub r#type: String,
        #[serde(flatten)]
        pub payload: PresenceInnerPayload,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(tag = "event", content = "payload", rename_all = "snake_case")]
    pub enum PresenceInnerPayload {
        Track(simd_json::OwnedValue),
    }

    #[cfg(test)]
    #[expect(clippy::unwrap_used, reason = "Allowed in test code for simplicity")]
    mod tests {
        use pretty_assertions::assert_eq;

        use super::*;
        use crate::message::{ProtocolMessage, ProtocolPayload};

        #[test]
        fn test_presence_track_deserialization() {
            let json_data = r#"
            {
                "topic": "realtime:af",
                "event": "presence",
                "payload": {
                    "type": "presence",
                    "event": "track",
                    "payload": {
                        "message": "bbbbbbb"
                    }
                },
                "ref": "27",
                "join_ref": "1"
            }
            "#;

            let expected_struct = ProtocolMessage {
                topic: "realtime:af".to_owned(),
                payload: ProtocolPayload::PresenceInner(PresenceInner {
                    r#type: "presence".to_owned(),
                    payload: PresenceInnerPayload::Track(simd_json::json!({"message": "bbbbbbb"})),
                }),
                ref_field: Some("27".to_owned()),
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
    #[expect(clippy::unwrap_used, reason = "Allowed in test code for simplicity")]
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
        pub joins: HashMap<String, super::presence_state::Presence>,
        pub leaves: HashMap<String, super::presence_state::Presence>,
    }

    #[cfg(test)]
    #[expect(clippy::unwrap_used, reason = "Allowed in test code for simplicity")]
    mod tests {
        use pretty_assertions::assert_eq;

        use super::*;
        use crate::message::{
            ProtocolMessage, ProtocolPayload,
            presence_state::{Presence, PresenceMeta},
        };

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
                                    "t": 21957173
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
                                    name: Some("service_role_47".to_owned()),
                                    payload: simd_json::json!({"t": 21_957_173_u64 }),
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
    #[expect(clippy::unwrap_used, reason = "Allowed in test code for simplicity")]
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
    #[expect(clippy::unwrap_used, reason = "Allowed in test code for simplicity")]
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
    pub struct PhxClose;

    #[cfg(test)]
    #[expect(clippy::unwrap_used, reason = "Allowed in test code for simplicity")]
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
                payload: ProtocolPayload::PhxClose(PhxClose),
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
    #[expect(clippy::unwrap_used, reason = "Allowed in test code for simplicity")]
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
    #[expect(clippy::unwrap_used, reason = "Allowed in test code for simplicity")]
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

    use serde::{Deserialize, Serialize};

    #[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Buffer(pub Vec<u8>);

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
        #[serde(skip_serializing_if = "Option::is_none")]
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
        ///
        /// # Errors
        ///
        /// Returns an error if deserialization of the record fails.
        pub fn parse_record<T: serde::de::DeserializeOwned>(
            self,
        ) -> Result<Data<T, O>, simd_json::Error> {
            let record = match self.record {
                Some(buffer) => {
                    let mut data = buffer.0;
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
        ///
        /// # Errors
        ///
        /// Returns an error if deserialization of the `old_record` fails.
        pub fn parse_old_record<K: serde::de::DeserializeOwned>(
            self,
        ) -> Result<Data<R, K>, simd_json::Error> {
            let old_record = match self.old_record {
                Some(buffer) => {
                    let mut data = buffer.0;
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
}
