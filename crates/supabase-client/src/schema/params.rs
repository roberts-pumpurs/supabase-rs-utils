//! Pure `PostgREST` query pairs; pass these directly to an HTTP query serializer.
//!
//! Runtime table names need no generated relation marker:
//! ```rust,ignore
//! let table = "skills";
//! let mut pairs = vec![
//!     params::select("*"),
//!     params::order_by("created_at", params::Order::Desc),
//! ];
//! pairs.extend(params::range(0, 24)?);
//! let response = http.get(format!("{base}/rest/v1/{table}"))
//!     .query(&pairs).send().await?;
//! ```
//! Values are unencoded; the HTTP serializer performs URL encoding exactly once.
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

/// An inclusive range that cannot be represented as an offset and `usize` limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeError {
    /// The upper bound precedes the lower bound.
    Reversed,
    /// The inclusive row count exceeds `usize::MAX`.
    Overflow,
}

impl fmt::Display for RangeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Reversed => "inclusive range upper bound precedes lower bound",
            Self::Overflow => "inclusive range row count exceeds usize::MAX",
        })
    }
}

impl core::error::Error for RangeError {}

/// Render an unencoded runtime selection expression, such as `"*"` or `"id,name"`.
///
/// This accepts `PostgREST` selection grammar, not a literal identifier.
#[must_use]
pub fn select(selection: &str) -> QueryPair {
    (Cow::Borrowed("select"), Cow::Owned(selection.into()))
}

/// Render a response row limit. Zero requests zero rows.
#[must_use]
pub fn limit(count: usize) -> QueryPair {
    (Cow::Borrowed("limit"), Cow::Owned(count.to_string()))
}

/// Render a zero-based response row offset.
#[must_use]
pub fn offset(count: usize) -> QueryPair {
    (Cow::Borrowed("offset"), Cow::Owned(count.to_string()))
}

/// Render an inclusive response range as `[offset(low), limit(high - low + 1)]`.
///
/// # Errors
/// Returns [`RangeError::Reversed`] if `high < low`, or [`RangeError::Overflow`]
/// if the inclusive row count cannot fit in `usize`.
pub fn range(low: usize, high: usize) -> Result<[QueryPair; 2], RangeError> {
    let count = high
        .checked_sub(low)
        .ok_or(RangeError::Reversed)?
        .checked_add(1)
        .ok_or(RangeError::Overflow)?;
    Ok([offset(low), limit(count)])
}

/// Prefix a pair with a runtime embedded relation path.
///
/// Each path element is one literal selected relation name or alias, not dotted
/// grammar or a table URL. For example, `scope(&["tasks"], limit(5))` limits
/// embedded `tasks` rows, not root rows. Nested paths are escaped segment by
/// segment. An empty path leaves the pair unchanged. No generated marker is needed.
#[must_use]
pub fn scope(path: &[&str], pair: QueryPair) -> QueryPair {
    if path.is_empty() {
        return pair;
    }
    let mut key = String::new();
    for name in path {
        super::relationship::write_identifier(&mut key, name);
        key.push('.');
    }
    key.push_str(&pair.0);
    (Cow::Owned(key), pair.1)
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
#[expect(
    clippy::expect_used,
    reason = "String writes cannot fail; retain the documented Display failure semantics."
)]
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

/// Render ordering by one runtime literal column name, escaping grammar punctuation.
///
/// Expressions, dotted paths, and pre-escaped names are treated as literal names.
#[must_use]
pub fn order_by(column: &str, direction: Order) -> QueryPair {
    runtime_order(column, direction, None)
}

/// Render runtime literal column ordering with explicit null placement.
#[must_use]
pub fn order_by_with_nulls(column: &str, direction: Order, nulls: Nulls) -> QueryPair {
    runtime_order(column, direction, Some(nulls))
}

fn runtime_order(column: &str, direction: Order, nulls: Option<Nulls>) -> QueryPair {
    let (direction, nulls) = order_suffix(direction, nulls);
    let capacity = super::relationship::identifier_len(column)
        .saturating_add(direction.len())
        .saturating_add(nulls.len());
    let mut value = String::with_capacity(capacity);
    super::relationship::write_identifier(&mut value, column);
    value.push_str(direction);
    value.push_str(nulls);
    (Cow::Borrowed("order"), Cow::Owned(value))
}
fn order_with_nulls_impl<C: Column>(
    _column: C,
    direction: Order,
    nulls: Option<Nulls>,
) -> QueryPair {
    let (direction, nulls) = order_suffix(direction, nulls);
    (
        Cow::Borrowed("order"),
        Cow::Owned([C::SELECT, direction, nulls].concat()),
    )
}

const fn order_suffix(direction: Order, nulls: Option<Nulls>) -> (&'static str, &'static str) {
    let direction = match direction {
        Order::Asc => ".asc",
        Order::Desc => ".desc",
    };
    let nulls = match nulls {
        Some(Nulls::First) => ".nullsfirst",
        Some(Nulls::Last) => ".nullslast",
        None => "",
    };
    (direction, nulls)
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
