//! Typed requests over the native checked `PostgREST` transport.
use super::Relation;
use alloc::borrow::Cow;
use core::{borrow::Borrow, fmt, marker::PhantomData};
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
/// A column storing JSON or JSONB.
pub trait JsonColumn: Column {}
/// A named result shape for one relation.
pub trait Projection<R: Relation>: DeserializeOwned {
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
/// A read request with response pagination; it cannot become a mutation.
pub struct Paged;
/// A request on which a mutation has already been chosen.
pub struct Write;
/// Selection may still be changed.
pub struct Unlocked;
/// Embedded predicates have fixed the projection.
pub struct Locked;
/// A typed request. The native builder is available only through `into_raw`.
#[must_use]
pub struct Query<R, P, State = Read, Selection = Unlocked> {
    builder: rp_postgrest::Builder,
    #[expect(
        clippy::type_complexity,
        reason = "Function markers retain type identity without imposing ownership or auto-trait bounds."
    )]
    marker: PhantomData<fn() -> (R, P, State, Selection)>,
}
/// Start a full-row request for the exact generated database identity.
#[expect(
    clippy::needless_pass_by_value,
    reason = "Query construction retains the consuming generated-client entry point"
)]
pub fn query<R: Relation + Projection<R>>(client: rp_postgrest::Postgrest) -> Query<R, R> {
    Query {
        builder: client.from(R::NAME).schema(R::SCHEMA),
        marker: PhantomData,
    }
}
macro_rules! comparison {
    ($method:ident, $doc:literal) => {
        #[doc = $doc]
        ///
        /// # Panics
        /// Panics if the scalar's `Display` implementation returns a formatting error.
        pub fn $method<C, V>(mut self, column: C, value: &V) -> Self
        where
            C: Column<Relation = R>,
            C::Filter: Borrow<V>,
            V: fmt::Display + ?Sized,
        {
            let (key, value) = super::params::$method(column, value);
            self.builder.append_query(key, value);
            self
        }
    };
}
impl<R: Relation, P: Projection<R>, State> Query<R, P, State, Unlocked> {
    /// Choose a result shape before embedded predicates lock selection.
    pub fn select<S: super::selection::Selection<Relation = R>>(
        self,
        _selection: S,
    ) -> Query<R, S::Record, State, Unlocked> {
        Query {
            builder: self.builder,
            marker: PhantomData,
        }
    }
}
impl<R: Relation, P: Projection<R>, State, Selection> Query<R, P, State, Selection> {
    /// Append scalar predicates scoped to a selected relationship.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "Value syntax accepts generated handles and temporary composed paths."
    )]
    pub fn embedded<H, F>(mut self, handle: H, apply: F) -> Query<R, P, State, Locked>
    where
        H: super::EmbedPath<Owner = P, Source = R>,
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
    pub fn exists<H: super::EmbedPath<Owner = P, Source = R>>(
        mut self,
        handle: H,
    ) -> Query<R, P, State, Locked> {
        let mut key = String::with_capacity(handle.path_len());
        handle.write_path(&mut key);
        self.builder.append_query(key, "not.is.null");
        self.lock()
    }
    /// Keep parents without a matching related row.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "Value syntax accepts generated handles and temporary composed paths."
    )]
    pub fn not_exists<H: super::EmbedPath<Owner = P, Source = R>>(
        mut self,
        handle: H,
    ) -> Query<R, P, State, Locked> {
        let mut key = String::with_capacity(handle.path_len());
        handle.write_path(&mut key);
        self.builder.append_query(key, "is.null");
        self.lock()
    }
    fn lock(self) -> Query<R, P, State, Locked> {
        Query {
            builder: self.builder,
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
    pub fn is_null<C: NullableColumn<Relation = R>>(mut self, column: C) -> Self {
        let (key, value) = super::params::is_null(column);
        self.builder.append_query(key, value);
        self
    }
    /// Compose typed ordering without changing mutation eligibility.
    pub fn order<C: Column<Relation = R>>(mut self, column: C, direction: super::Order) -> Self {
        self.builder = self
            .builder
            .order(super::params::order(column, direction).1.into_owned());
        self
    }
    /// Compose typed ordering with explicit null placement.
    pub fn order_with_nulls<C: Column<Relation = R>>(
        mut self,
        column: C,
        direction: super::Order,
        nulls: super::Nulls,
    ) -> Self {
        self.builder = self.builder.order(
            super::params::order_with_nulls(column, direction, nulls)
                .1
                .into_owned(),
        );
        self
    }
    /// Match literal scalar values in an IN list.
    ///
    /// # Panics
    /// Panics if a scalar's `Display` implementation returns a formatting error.
    pub fn in_<'a, C, V, I>(mut self, column: C, values: I) -> Self
    where
        C: Column<Relation = R>,
        C::Filter: Borrow<V>,
        V: fmt::Display + ?Sized + 'a,
        I: IntoIterator<Item = &'a V>,
    {
        let (key, value) = super::params::in_(column, values);
        self.builder.append_query(key, value);
        self
    }
    /// Compare a JSON text path; an empty path is rejected.
    ///
    /// # Errors
    /// Returns [`rp_postgrest::ConfigError::EmptyJsonPath`] when `path` is empty.
    pub fn json_text_eq<C: JsonColumn<Relation = R>>(
        mut self,
        column: C,
        path: &[&str],
        value: &str,
    ) -> Result<Self, rp_postgrest::Error> {
        let (key, value) = super::params::json_text_eq(column, path, value)?;
        self.builder.append_query(key, value);
        Ok(self)
    }
    /// Drop typed guarantees, applying the final projection.
    pub fn into_raw(self) -> rp_postgrest::Builder {
        self.builder.select(P::selection())
    }
    /// Execute and decode the selected rows.
    ///
    /// # Errors
    /// Preserves serialization, checked transport, body-read and decode failures.
    pub async fn fetch(self) -> Result<Vec<P>, rp_postgrest::Error> {
        self.into_raw().fetch().await
    }
    /// Execute with server single-row cardinality semantics.
    ///
    /// # Errors
    /// Preserves serialization, checked transport, body-read and decode failures.
    pub async fn fetch_one(self) -> Result<P, rp_postgrest::Error> {
        self.into_raw().single().fetch().await
    }
}
impl<R: WritableRelation, P: Projection<R>, Selection> Query<R, P, Read, Selection> {
    /// Insert a generated table payload, deferring serialization errors until execution.
    pub fn insert(self, payload: &R::Insert) -> Query<R, P, Write, Selection> {
        Query {
            builder: self.builder.insert_json(payload),
            marker: PhantomData,
        }
    }
    /// Update using a generated table payload.
    pub fn update(self, payload: &R::Update) -> Query<R, P, Write, Selection> {
        Query {
            builder: self.builder.update_json(payload),
            marker: PhantomData,
        }
    }
    /// Delete matching rows. A second mutation cannot be selected.
    pub fn delete(self) -> Query<R, P, Write, Selection> {
        Query {
            builder: self.builder.delete(),
            marker: PhantomData,
        }
    }
}

/// A borrowed filter scope; it cannot execute a separate child request.
pub struct ScopedFilters<'a, R, P> {
    builder: &'a mut rp_postgrest::Builder,
    prefix: &'a mut String,
    marker: PhantomData<fn() -> (R, P)>,
}
macro_rules! scoped_comparison {
    ($method:ident, $doc:literal) => {
        #[doc = $doc]
        ///
        /// # Panics
        /// Panics if the scalar's `Display` implementation returns a formatting error.
        pub fn $method<C, V>(&mut self, column: C, value: &V) -> &mut Self
        where
            C: Column<Relation = R>,
            C::Filter: Borrow<V>,
            V: fmt::Display + ?Sized,
        {
            self.append(super::params::$method(column, value));
            self
        }
    };
}
#[expect(
    clippy::arithmetic_side_effects,
    reason = "Allocated paths and static SQL identifiers fit usize capacity arithmetic."
)]
impl<R: Relation, P> ScopedFilters<'_, R, P> {
    fn append(&mut self, (column, value): super::QueryPair) {
        let mut key = String::with_capacity(self.prefix.len() + 1 + column.len());
        key.push_str(self.prefix);
        key.push('.');
        key.push_str(&column);
        self.builder.append_query(key, value);
    }
    /// Match child literal scalar values.
    ///
    /// # Panics
    /// Panics if a scalar's `Display` implementation returns a formatting error.
    pub fn in_<'a, C, V, I>(&mut self, column: C, values: I) -> &mut Self
    where
        C: Column<Relation = R>,
        C::Filter: Borrow<V>,
        V: fmt::Display + ?Sized + 'a,
        I: IntoIterator<Item = &'a V>,
    {
        self.append(super::params::in_(column, values));
        self
    }
    /// Compare a child JSON text path; an empty path is rejected.
    ///
    /// # Errors
    /// Returns [`rp_postgrest::ConfigError::EmptyJsonPath`] when `path` is empty.
    pub fn json_text_eq<C: JsonColumn<Relation = R>>(
        &mut self,
        column: C,
        path: &[&str],
        value: &str,
    ) -> Result<&mut Self, rp_postgrest::Error> {
        self.append(super::params::json_text_eq(column, path, value)?);
        Ok(self)
    }
    scoped_comparison!(eq, "Compare a child scalar for equality.");
    scoped_comparison!(neq, "Compare a child scalar for inequality.");
    scoped_comparison!(gt, "Compare a child scalar using greater-than.");
    scoped_comparison!(gte, "Compare a child scalar using greater-than-or-equal.");
    scoped_comparison!(lt, "Compare a child scalar using less-than.");
    scoped_comparison!(lte, "Compare a child scalar using less-than-or-equal.");
    /// Test a nullable child column.
    pub fn is_null<C: NullableColumn<Relation = R>>(&mut self, column: C) -> &mut Self {
        self.append(super::params::is_null(column));
        self
    }
    /// Append filters in a further selected child scope.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "Value syntax accepts generated handles and temporary composed paths."
    )]
    pub fn embedded<H, F>(&mut self, handle: H, apply: F) -> &mut Self
    where
        H: super::EmbedPath<Owner = P, Source = R>,
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
    pub fn exists<H: super::EmbedPath<Owner = P, Source = R>>(&mut self, handle: H) -> &mut Self {
        self.presence(handle, "not.is.null")
    }
    /// Require no matching selected child.
    pub fn not_exists<H: super::EmbedPath<Owner = P, Source = R>>(
        &mut self,
        handle: H,
    ) -> &mut Self {
        self.presence(handle, "is.null")
    }
    #[expect(
        clippy::needless_pass_by_value,
        reason = "Presence helpers consume the same handle syntax as public predicates."
    )]
    fn presence<H: super::EmbedPath<Owner = P, Source = R>>(
        &mut self,
        handle: H,
        value: &'static str,
    ) -> &mut Self {
        let mut key = String::with_capacity(self.prefix.len() + 1 + handle.path_len());
        key.push_str(self.prefix);
        key.push('.');
        handle.write_path(&mut key);
        self.builder.append_query(key, value);
        self
    }
}

macro_rules! read_operations {
    ($state:ty) => {
        impl<R: Relation, P: Projection<R>, Selection> Query<R, P, $state, Selection> {
            /// Paginate the response; the returned query cannot become a mutation.
            pub fn limit(self, count: usize) -> Query<R, P, Paged, Selection> {
                Query {
                    builder: self.builder.limit(count),
                    marker: PhantomData,
                }
            }
            /// Request an inclusive response range; cannot become a mutation.
            pub fn range(self, low: usize, high: usize) -> Query<R, P, Paged, Selection> {
                Query {
                    builder: self.builder.range(low, high),
                    marker: PhantomData,
                }
            }
            /// Fetch rows and the server's total.
            ///
            /// # Errors
            /// Preserves checked execution and decoding failures, and rejects missing
            /// or invalid server count metadata.
            pub async fn fetch_with_count(
                self,
                count: super::Count,
            ) -> Result<super::Counted<Vec<P>>, rp_postgrest::Error> {
                self.into_raw().fetch_with_count(count).await
            }
            /// Count matching rows without decoding a row body.
            ///
            /// # Errors
            /// Preserves checked execution failures and rejects missing or invalid
            /// server count metadata.
            pub async fn count(self, count: super::Count) -> Result<u64, rp_postgrest::Error> {
                self.into_raw().execute_count(count).await
            }
        }
    };
}
read_operations!(Read);
read_operations!(Paged);

impl<R: Relation, P: Projection<R>, Selection> Query<R, P, Write, Selection> {
    /// Execute a minimal-return mutation without JSON decoding.
    ///
    /// # Errors
    /// Preserves serialization, checked transport and server-response failures.
    pub async fn execute(self) -> Result<(), rp_postgrest::Error> {
        self.into_raw().return_minimal().execute_checked().await?;
        Ok(())
    }
    /// Execute a minimal-return mutation and read the affected-row total.
    ///
    /// # Errors
    /// Preserves checked execution failures and rejects missing or invalid
    /// server count metadata.
    pub async fn execute_with_count(self, count: super::Count) -> Result<u64, rp_postgrest::Error> {
        self.into_raw().execute_count(count).await
    }
}
