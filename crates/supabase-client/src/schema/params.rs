//! Pure typed `PostgREST` query pairs; pass these directly to an HTTP query serializer.
use super::{Column, JsonColumn, NullableColumn, Projection, Relation};
use alloc::borrow::Cow;
use core::{
    borrow::Borrow,
    fmt::{self, Write as _},
};

/// An unencoded query key and value.
pub type QueryPair = (Cow<'static, str>, Cow<'static, str>);
/// Sort direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    /// Sort in ascending order.
    Asc,
    /// Sort in descending order.
    Desc,
}
/// Explicit SQL null placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nulls {
    /// Place null values before non-null values.
    First,
    /// Place null values after non-null values.
    Last,
}

/// Render a named selection.
#[must_use]
pub fn projection<R: Relation, P: Projection<R>>() -> QueryPair {
    (Cow::Borrowed("select"), P::selection())
}
macro_rules! comparison {
    ($($method:ident),*) => {$ (
        #[doc = concat!("Render a `", stringify!($method), "` scalar predicate.")]
        ///
        /// # Panics
        /// Panics if the scalar's `Display` implementation returns a formatting error.
        #[must_use]
        pub fn $method<C: Column, V: fmt::Display + ?Sized>(_column: C, value: &V) -> QueryPair
        where C::Filter: Borrow<V> {
            (Cow::Borrowed(C::SELECT), Cow::Owned(format!("{}.{value}", stringify!($method))))
        }
    )*};
}
comparison!(eq, neq, gt, gte, lt, lte);
/// Render a SQL null predicate.
#[must_use]
pub const fn is_null<C: NullableColumn>(_column: C) -> QueryPair {
    (Cow::Borrowed(C::SELECT), Cow::Borrowed("is.null"))
}
struct Escaped<'a>(&'a mut String);
impl fmt::Write for Escaped<'_> {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        for character in value.chars() {
            if character == '"' || character == '\\' {
                self.0.push('\\');
            }
            self.0.push(character);
        }
        Ok(())
    }
}
/// Render literal IN values, quoting and escaping in list context only.
///
/// # Panics
/// Panics if a scalar's `Display` implementation returns a formatting error.
#[must_use]
pub fn in_<'a, C, V, I>(_column: C, values: I) -> QueryPair
where
    C: Column,
    C::Filter: Borrow<V>,
    V: fmt::Display + ?Sized + 'a,
    I: IntoIterator<Item = &'a V>,
{
    let mut output = String::from("in.(");
    let mut first = true;
    for value in values {
        if !first {
            output.push(',');
        }
        first = false;
        output.push('"');
        // String writes cannot fail; retain Display's normal formatting failure semantics.
        write!(Escaped(&mut output), "{value}")
            .expect("scalar Display failed while rendering IN values");
        output.push('"');
    }
    output.push(')');
    (Cow::Borrowed(C::SELECT), Cow::Owned(output))
}
/// Render one ordering term.
#[must_use]
pub fn order<C: Column>(column: C, direction: Order) -> QueryPair {
    order_with_nulls_impl(column, direction, None)
}
/// Render one ordering term with explicit null placement.
#[must_use]
pub fn order_with_nulls<C: Column>(column: C, direction: Order, nulls: Nulls) -> QueryPair {
    order_with_nulls_impl(column, direction, Some(nulls))
}
fn order_with_nulls_impl<C: Column>(
    _column: C,
    direction: Order,
    nulls: Option<Nulls>,
) -> QueryPair {
    let direction = match direction {
        Order::Asc => ".asc",
        Order::Desc => ".desc",
    };
    let nulls = match nulls {
        Some(Nulls::First) => ".nullsfirst",
        Some(Nulls::Last) => ".nullslast",
        None => "",
    };
    let value = [C::SELECT, direction, nulls].concat();
    (Cow::Borrowed("order"), Cow::Owned(value))
}
/// Render equality on a JSON text path; path identifiers are escaped, values stay literal.
///
/// # Errors
/// Returns [`rp_postgrest::ConfigError::EmptyJsonPath`] when `path` is empty.
pub fn json_text_eq<C: JsonColumn>(
    _column: C,
    path: &[&str],
    value: &str,
) -> Result<QueryPair, rp_postgrest::Error> {
    let Some((last, parents)) = path.split_last() else {
        return Err(rp_postgrest::ConfigError::EmptyJsonPath.into());
    };
    let mut key = String::from(C::SELECT);
    for parent in parents {
        key.push_str("->");
        super::relationship::write_identifier(&mut key, parent);
    }
    key.push_str("->>");
    super::relationship::write_identifier(&mut key, last);
    Ok((Cow::Owned(key), Cow::Owned(format!("eq.{value}"))))
}
