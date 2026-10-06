//! Serialization and `PostgREST` integration for generated database bindings.

use alloc::borrow::Cow;
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};

use serde::{Deserialize, Serialize, Serializer};

const PATH_SEGMENT: &AsciiSet = &NON_ALPHANUMERIC.remove(b'-').remove(b'_').remove(b'~');

/// A write field that distinguishes an absent key from an explicit value.
/// Use `Field<Option<T>>` when the database permits an explicit JSON null.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Field<T> {
    /// Leave the key out of the request, preserving database defaults.
    #[default]
    Omit,
    /// Send this value (including null when `T` is an `Option`).
    Value(T),
}

impl<T> Field<T> {
    /// Whether a generated serializer should omit this field.
    #[must_use]
    pub const fn is_omit(&self) -> bool {
        matches!(self, Self::Omit)
    }
}

impl<T: Serialize> Serialize for Field<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Value(value) => value.serialize(serializer),
            Self::Omit => Err(serde::ser::Error::custom(
                "Field::Omit must be skipped by the containing field serializer",
            )),
        }
    }
}

/// `PostgreSQL` arrays support nullable elements and variable rank.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Array<T> {
    /// A one-dimensional array, including null elements.
    Elements(Vec<Option<T>>),
    /// A higher-dimensional array.
    Nested(Vec<Array<T>>),
}

/// A generated relation's exact database identity.
pub trait Relation {
    /// `PostgreSQL` schema name.
    const SCHEMA: &'static str;
    /// `PostgreSQL` relation name.
    const NAME: &'static str;
}

/// A generated function's identity and request/response types.
pub trait Function {
    /// Serializable RPC argument object.
    type Args: Serialize;
    /// Response decoded using `serde_json`.
    type Returns;
    /// `PostgreSQL` schema name.
    const SCHEMA: &'static str;
    /// `PostgreSQL` function name.
    const NAME: &'static str;
}

/// Start a relation request, retaining the native `PostgREST` builder.
#[must_use]
pub fn from<R: Relation>(client: rp_postgrest::Postgrest) -> rp_postgrest::Builder {
    let name: Cow<'_, str> = utf8_percent_encode(R::NAME, PATH_SEGMENT).into();
    client.schema(R::SCHEMA).from(name)
}

/// Start an RPC request, preserving omitted arguments and explicit nulls.
///
/// # Errors
/// Returns the argument serialization error, including an unskipped omitted field.
pub fn rpc<F: Function>(
    client: rp_postgrest::Postgrest,
    args: &F::Args,
) -> Result<rp_postgrest::Builder, serde_json::Error> {
    let body = serde_json::to_string(args)?;
    let name: Cow<'_, str> = utf8_percent_encode(F::NAME, PATH_SEGMENT).into();
    Ok(client.schema(F::SCHEMA).rpc(name, body))
}

/// Include bindings produced by a consumer's build script in `OUT_DIR`.
#[macro_export]
macro_rules! include_schema {
    ($filename:literal) => {
        include!(concat!(env!("OUT_DIR"), "/", $filename));
    };
}

/// Common generated binding runtime imports.
pub mod prelude {
    pub use super::{Array, Field, Function, Relation, from, rpc};
    pub use crate::include_schema;
}

#[cfg(test)]
#[expect(
    clippy::unwrap_used,
    reason = "Test failures should retain the underlying serialization error."
)]
mod tests {
    use super::{Array, Field};
    use serde::Serialize;
    use serde_json::{Value, json};

    #[derive(Default, Serialize)]
    struct Write {
        #[serde(skip_serializing_if = "Field::is_omit")]
        nullable: Field<Option<String>>,
        #[serde(skip_serializing_if = "Field::is_omit")]
        required: Field<String>,
    }

    #[test]
    fn omission_null_and_values_are_distinct() {
        assert_eq!(serde_json::to_value(Write::default()).unwrap(), json!({}));
        let mut write = Write {
            nullable: Field::Value(None),
            ..Write::default()
        };
        assert_eq!(
            serde_json::to_value(&write).unwrap(),
            json!({"nullable": null})
        );
        write.nullable = Field::Value(Some("present".into()));
        write.required = Field::Value("required".into());
        assert_eq!(
            serde_json::to_value(write).unwrap(),
            json!({"nullable": "present", "required": "required"})
        );
    }

    #[test]
    fn omission_outside_a_skipped_field_is_an_error() {
        serde_json::to_value(Field::<String>::Omit).unwrap_err();
        serde_json::to_value(vec![Field::<String>::Omit]).unwrap_err();
        assert_eq!(
            serde_json::to_value(Field::<Option<String>>::Value(None)).unwrap(),
            Value::Null
        );
    }

    #[test]
    fn arrays_preserve_rank_null_elements_and_empty_arrays() {
        for input in [
            json!([]),
            json!([1_i32, null, 3_i32]),
            json!([[1_i32, null], [2_i32, 3_i32]]),
            json!([[[], []], [[], []]]),
        ] {
            let array: Array<i32> = serde_json::from_value(input.clone()).unwrap();
            assert_eq!(serde_json::to_value(array).unwrap(), input);
        }
        serde_json::from_value::<Array<i32>>(json!([1_i32, [2_i32]])).unwrap_err();
        serde_json::from_value::<Array<i32>>(Value::Null).unwrap_err();
    }
}
