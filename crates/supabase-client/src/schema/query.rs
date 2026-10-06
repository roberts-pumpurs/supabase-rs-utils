//! Typed requests over the native checked `PostgREST` transport.
use super::{PATH_SEGMENT, Relation};
use alloc::borrow::Cow;
use core::{borrow::Borrow, fmt, marker::PhantomData};
use percent_encoding::utf8_percent_encode;
use serde::{Serialize, de::DeserializeOwned};

/// A base table's write payloads.
pub trait WritableRelation: Relation {
    /// Insert payload.
    type Insert: Serialize;
    /// Update payload.
    type Update: Serialize;
}
/// A generated column belonging to one relation.
pub trait Column: Copy {
    /// Owning row type.
    type Relation: Relation;
    /// Exact decoded field type.
    type Value: DeserializeOwned;
    /// Non-null scalar filter type.
    type Filter: ?Sized;
    /// Exact SQL name and response key.
    const NAME: &'static str;
    /// Escaped `PostgREST` grammar identifier.
    const SELECT: &'static str;
}
/// A column which permits SQL null.
pub trait NullableColumn: Column {}
/// A named result shape for one relation.
pub trait Projection: DeserializeOwned {
    /// Owning row type.
    type Relation: Relation;
    /// Selection sent at execution time.
    const SELECT_LEN: usize;
    /// Append selection into a shared parent buffer.
    fn write_selection(output: &mut String);
    /// Build the selection in a single exactly-sized buffer.
    #[must_use]
    fn selection() -> Cow<'static, str> {
        let mut output = String::with_capacity(Self::SELECT_LEN);
        Self::write_selection(&mut output);
        Cow::Owned(output)
    }
}
/// A request on which a mutation has not yet been chosen.
pub struct Read;
/// A request on which a mutation has already been chosen.
pub struct Write;
/// Selection may still be changed.
pub struct Unlocked;
/// Embedded predicates have fixed the projection.
pub struct Locked;
/// A typed request. The native builder is available only through `into_raw`.
#[must_use]
pub struct Query<R, P, State = Read, Selection = Unlocked> {
    builder: postgrest::Builder,
    serialization_error: Option<serde_json::Error>,
    #[expect(
        clippy::type_complexity,
        reason = "Function markers retain type identity without imposing ownership or auto-trait bounds."
    )]
    marker: PhantomData<fn() -> (R, P, State, Selection)>,
}
/// Start a full-row request for the exact generated database identity.
pub fn query<R: Relation + Projection<Relation = R>>(client: postgrest::Postgrest) -> Query<R, R> {
    let name: Cow<'_, str> = utf8_percent_encode(R::NAME, PATH_SEGMENT).into();
    Query {
        builder: client.schema(R::SCHEMA).from(name),
        serialization_error: None,
        marker: PhantomData,
    }
}
/// Failure to serialize, execute, read, or decode a typed request.
#[derive(Debug)]
pub enum QueryError {
    /// Request payload could not be serialized.
    Serialization(serde_json::Error),
    /// Native checked execution failed, preserving status and response metadata.
    Execution(postgrest::ExecuteError),
    /// A successful response body could not be read.
    ResponseBody(postgrest::reqwest::Error),
    /// A successful response did not match the selected shape.
    Decode(serde_json::Error),
}
impl fmt::Display for QueryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Serialization(error) => {
                write!(formatter, "request serialization failed: {error}")
            }
            Self::Execution(error) => write!(formatter, "request execution failed: {error}"),
            Self::ResponseBody(error) => write!(formatter, "response body read failed: {error}"),
            Self::Decode(error) => write!(formatter, "response decoding failed: {error}"),
        }
    }
}
impl core::error::Error for QueryError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Serialization(error) | Self::Decode(error) => Some(error),
            Self::Execution(error) => Some(error),
            Self::ResponseBody(error) => Some(error),
        }
    }
}
macro_rules! comparison {
    ($method:ident, $doc:literal) => {
        #[doc = $doc]
        pub fn $method<C, V>(mut self, _column: C, value: &V) -> Self
        where
            C: Column<Relation = R>,
            C::Filter: Borrow<V>,
            V: fmt::Display + ?Sized,
        {
            // Scalar filters consume the remaining value literally. Only the URL encodes it.
            let value = format!("{}.{value}", stringify!($method));
            self.builder.queries.push((C::SELECT.to_owned(), value));
            self
        }
    };
}
impl<R: Relation, P: Projection<Relation = R>, State> Query<R, P, State, Unlocked> {
    /// Choose a result shape before embedded predicates lock selection.
    pub fn select<Q: Projection<Relation = R>>(self) -> Query<R, Q, State, Unlocked> {
        Query {
            builder: self.builder,
            serialization_error: self.serialization_error,
            marker: PhantomData,
        }
    }
}
impl<R: Relation, P: Projection<Relation = R>, State, Selection> Query<R, P, State, Selection> {
    /// Append scalar predicates scoped to a selected relationship.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "Value syntax accepts generated handles and temporary composed paths."
    )]
    pub fn embedded<H, F>(mut self, handle: H, apply: F) -> Query<R, P, State, Locked>
    where
        H: super::EmbedPath<Owner = P>,
        F: FnOnce(&mut ScopedFilters<'_, H::Target, H::Selected>),
    {
        let mut prefix = String::with_capacity(handle.path_len());
        handle.write_path(&mut prefix);
        apply(&mut ScopedFilters {
            builder: &mut self.builder,
            prefix: &mut prefix,
            marker: PhantomData,
        });
        self.lock()
    }
    /// Keep parents with a matching related row.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "Value syntax accepts generated handles and temporary composed paths."
    )]
    pub fn exists<H: super::EmbedPath<Owner = P>>(
        mut self,
        handle: H,
    ) -> Query<R, P, State, Locked> {
        let mut key = String::with_capacity(handle.path_len());
        handle.write_path(&mut key);
        self.builder.queries.push((key, "not.is.null".to_owned()));
        self.lock()
    }
    /// Keep parents without a matching related row.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "Value syntax accepts generated handles and temporary composed paths."
    )]
    pub fn not_exists<H: super::EmbedPath<Owner = P>>(
        mut self,
        handle: H,
    ) -> Query<R, P, State, Locked> {
        let mut key = String::with_capacity(handle.path_len());
        handle.write_path(&mut key);
        self.builder.queries.push((key, "is.null".to_owned()));
        self.lock()
    }
    fn lock(self) -> Query<R, P, State, Locked> {
        Query {
            builder: self.builder,
            serialization_error: self.serialization_error,
            marker: PhantomData,
        }
    }
    comparison!(eq, "Compare a column for equality with a non-null scalar.");
    comparison!(
        neq,
        "Compare a column for inequality with a non-null scalar."
    );
    comparison!(gt, "Compare a column using greater-than.");
    comparison!(gte, "Compare a column using greater-than-or-equal.");
    comparison!(lt, "Compare a column using less-than.");
    comparison!(lte, "Compare a column using less-than-or-equal.");
    /// Test a nullable column for SQL null.
    #[expect(
        clippy::wrong_self_convention,
        reason = "This query-builder predicate consumes the request, like the scalar comparisons."
    )]
    pub fn is_null<C: NullableColumn<Relation = R>>(mut self, _column: C) -> Self {
        self.builder = self.builder.is(C::SELECT, "null");
        self
    }
    /// Drop typed guarantees, applying the final projection.
    ///
    /// # Errors
    /// Returns any deferred payload serialization error.
    pub fn into_raw(self) -> Result<postgrest::Builder, QueryError> {
        if let Some(error) = self.serialization_error {
            return Err(QueryError::Serialization(error));
        }
        Ok(self.builder.select(P::selection().into_owned()))
    }
    /// Execute and decode the selected rows.
    ///
    /// # Errors
    /// Preserves serialization, checked transport, body-read and decode failures.
    pub async fn fetch(self) -> Result<Vec<P>, QueryError> {
        let response = self
            .into_raw()?
            .execute_checked()
            .await
            .map_err(QueryError::Execution)?;
        let body = response.bytes().await.map_err(QueryError::ResponseBody)?;
        serde_json::from_slice(&body).map_err(QueryError::Decode)
    }
    /// Execute with native single-row response semantics.
    ///
    /// # Errors
    /// Preserves serialization, checked transport, body-read and decode failures.
    pub async fn fetch_one(self) -> Result<P, QueryError> {
        let response = self
            .into_raw()?
            .single()
            .execute_checked()
            .await
            .map_err(QueryError::Execution)?;
        let body = response.bytes().await.map_err(QueryError::ResponseBody)?;
        serde_json::from_slice(&body).map_err(QueryError::Decode)
    }
}
impl<R: WritableRelation, P: Projection<Relation = R>, Selection> Query<R, P, Read, Selection> {
    fn mutate<T: Serialize>(
        self,
        payload: &T,
        apply: impl FnOnce(postgrest::Builder, String) -> postgrest::Builder,
    ) -> Query<R, P, Write, Selection> {
        let (builder, serialization_error) = match serde_json::to_string(payload) {
            Ok(body) => (apply(self.builder, body), None),
            Err(error) => (self.builder, Some(error)),
        };
        Query {
            builder,
            serialization_error,
            marker: PhantomData,
        }
    }
    /// Insert a generated table payload, deferring serialization errors until execution.
    pub fn insert(self, payload: &R::Insert) -> Query<R, P, Write, Selection> {
        self.mutate(payload, postgrest::Builder::insert)
    }
    /// Update using a generated table payload.
    pub fn update(self, payload: &R::Update) -> Query<R, P, Write, Selection> {
        self.mutate(payload, postgrest::Builder::update)
    }
    /// Delete matching rows. A second mutation cannot be selected.
    pub fn delete(self) -> Query<R, P, Write, Selection> {
        Query {
            builder: self.builder.delete(),
            serialization_error: self.serialization_error,
            marker: PhantomData,
        }
    }
}

/// A borrowed filter scope; it cannot execute a separate child request.
pub struct ScopedFilters<'a, R, P> {
    builder: &'a mut postgrest::Builder,
    prefix: &'a mut String,
    marker: PhantomData<fn() -> (R, P)>,
}
macro_rules! scoped_comparison {
    ($method:ident, $doc:literal) => {
        #[doc = $doc]
        pub fn $method<C, V>(&mut self, _column: C, value: &V) -> &mut Self
        where
            C: Column<Relation = R>,
            C::Filter: Borrow<V>,
            V: fmt::Display + ?Sized,
        {
            let mut key = String::with_capacity(self.prefix.len() + 1 + C::SELECT.len());
            key.push_str(self.prefix);
            key.push('.');
            key.push_str(C::SELECT);
            self.builder
                .queries
                .push((key, format!("{}.{value}", stringify!($method))));
            self
        }
    };
}
#[expect(
    clippy::arithmetic_side_effects,
    reason = "Allocated paths and static SQL identifiers fit usize capacity arithmetic."
)]
impl<R: Relation, P> ScopedFilters<'_, R, P> {
    scoped_comparison!(eq, "Compare a child scalar for equality.");
    scoped_comparison!(neq, "Compare a child scalar for inequality.");
    scoped_comparison!(gt, "Compare a child scalar using greater-than.");
    scoped_comparison!(gte, "Compare a child scalar using greater-than-or-equal.");
    scoped_comparison!(lt, "Compare a child scalar using less-than.");
    scoped_comparison!(lte, "Compare a child scalar using less-than-or-equal.");
    /// Test a nullable child column.
    pub fn is_null<C: NullableColumn<Relation = R>>(&mut self, _column: C) -> &mut Self {
        let mut key = String::with_capacity(self.prefix.len() + 1 + C::SELECT.len());
        key.push_str(self.prefix);
        key.push('.');
        key.push_str(C::SELECT);
        self.builder.queries.push((key, "is.null".to_owned()));
        self
    }
    /// Append filters in a further selected child scope.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "Value syntax accepts generated handles and temporary composed paths."
    )]
    pub fn embedded<H, F>(&mut self, handle: H, apply: F) -> &mut Self
    where
        H: super::EmbedPath<Owner = P>,
        F: FnOnce(&mut ScopedFilters<'_, H::Target, H::Selected>),
    {
        let length = self.prefix.len();
        self.prefix.reserve_exact(1 + handle.path_len());
        self.prefix.push('.');
        handle.write_path(self.prefix);
        apply(&mut ScopedFilters {
            builder: self.builder,
            prefix: self.prefix,
            marker: PhantomData,
        });
        self.prefix.truncate(length);
        self
    }
    /// Require a matching selected child.
    pub fn exists<H: super::EmbedPath<Owner = P>>(&mut self, handle: H) -> &mut Self {
        self.presence(handle, "not.is.null")
    }
    /// Require no matching selected child.
    pub fn not_exists<H: super::EmbedPath<Owner = P>>(&mut self, handle: H) -> &mut Self {
        self.presence(handle, "is.null")
    }
    #[expect(
        clippy::needless_pass_by_value,
        reason = "Presence helpers consume the same handle syntax as public predicates."
    )]
    fn presence<H: super::EmbedPath<Owner = P>>(&mut self, handle: H, value: &str) -> &mut Self {
        let mut key = String::with_capacity(self.prefix.len() + 1 + handle.path_len());
        key.push_str(self.prefix);
        key.push('.');
        handle.write_path(&mut key);
        self.builder.queries.push((key, value.to_owned()));
        self
    }
}
