mod authenticated;
mod unauthenticated;

use std::future::Future;

pub use authenticated::AuthenticatedSupabaseClient;
use reqwest::header;
use supabase_auth::SUPABASE_KEY;
use tracing::Span;
pub use unauthenticated::SupabaseClient;

use crate::{error, SupabaseClientError};

pub trait SupabaseClientExt {
    fn client(&mut self) -> impl Future<Output = reqwest::Client> + Send;
    fn supabase_url(&self) -> &url::Url;

    #[tracing::instrument(skip_all)]
    async fn execute<T: PostgRestQuery>(
        &mut self,
        query: T,
    ) -> Result<T::Output, error::SupabaseClientError> {
        let client = self.client().await;
        let query_builder = query.to_query()?;
        let method = query_builder.reqwest_method();
        let (path, body) = query_builder.build();
        let url = self.supabase_url().join("/rest/v1/")?.join(path.as_str())?;
        let current = Span::current();
        current.record("url", url.as_str());
        if let Some(body) = &body {
            current.record("body", format!("{body:#?}"));
        }

        let response = client.request(method, url).json(&body).send().await?;

        let status = response.status();
        let mut bytes = response.bytes().await?.to_vec();
        if status.is_success() {
            {
                let json = String::from_utf8_lossy(bytes.as_ref());
                tracing::info!(response_body = ?json, "Response JSON");
            }
            let result = simd_json::from_slice::<T::Output>(bytes.as_mut())?;
            Ok(result)
        } else {
            {
                let json = String::from_utf8_lossy(bytes.as_ref());
                tracing::error!(
                    status = %status,
                    body = %json,
                    "Failed to execute query"
                );
            }

            let result = simd_json::from_slice::<error::PostgrestError>(bytes.as_mut())?;

            Err(error::SupabaseClientError::PostgRestError(result))
        }
    }
}

impl SupabaseClientExt for AuthenticatedSupabaseClient {
    async fn client(&mut self) -> reqwest::Client {
        let client = self.client.read().await;
        client.clone()
    }

    fn supabase_url(&self) -> &url::Url {
        &self.supabase_url
    }
}

impl SupabaseClientExt for SupabaseClient {
    async fn client(&mut self) -> reqwest::Client {
        self.client.clone()
    }

    fn supabase_url(&self) -> &url::Url {
        &self.supabase_url
    }
}

pub(crate) fn construct_client(
    api_key: &str,
    bearer_token: &str,
) -> Result<reqwest::Client, SupabaseClientError> {
    let builder = reqwest::Client::builder();
    let mut headers = header::HeaderMap::new();
    headers.insert(SUPABASE_KEY, header::HeaderValue::from_str(api_key)?);
    headers.insert(
        header::AUTHORIZATION,
        header::HeaderValue::from_str(format!("Bearer {bearer_token}").as_str())?,
    );
    headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/json"),
    );
    let client = builder.default_headers(headers).build()?;
    Ok(client)
}

pub trait PostgRestQuery {
    type Output: serde::de::DeserializeOwned;

    fn to_query(&self) -> Result<query_builder::QueryBuilder, error::SupabaseClientError>;
}

mod query_builder {

    pub enum QueryBuilder {
        Post(PostQuery),
        Get(GetQuery),
        Patch(PatchQuery),
        Delete(DeleteQuery),
    }

    impl QueryBuilder {
        pub fn build(self) -> (String, Option<Vec<u8>>) {
            match self {
                QueryBuilder::Post(query) => query.build(),
                QueryBuilder::Get(query) => query.build(),
                QueryBuilder::Patch(query) => query.build(),
                QueryBuilder::Delete(query) => query.build(),
            }
        }

        pub fn reqwest_method(&self) -> reqwest::Method {
            match self {
                QueryBuilder::Post(_) => reqwest::Method::POST,
                QueryBuilder::Get(_) => reqwest::Method::GET,
                QueryBuilder::Patch(_) => reqwest::Method::PATCH,
                QueryBuilder::Delete(_) => reqwest::Method::DELETE,
            }
        }
    }

    pub struct PostQuery {
        pub filters: Vec<filter::Filter>,
        pub returning: Option<&'static str>,
        pub upsert: bool,
        pub body: Vec<u8>,
    }

    impl PostQuery {
        pub fn new(body: impl Into<Vec<u8>>) -> Self {
            PostQuery {
                filters: Vec::new(),
                returning: None,
                upsert: false,
                body: body.into(),
            }
        }

        pub fn filter(mut self, condition: filter::Filter) -> Self {
            self.filters.push(condition);
            self
        }

        pub fn returning(mut self, fields: &'static str) -> Self {
            self.returning = Some(fields);
            self
        }

        pub fn upsert(mut self, value: bool) -> Self {
            self.upsert = value;
            self
        }

        pub fn build(self) -> (String, Option<Vec<u8>>) {
            let mut params = Vec::new();

            for filter in self.filters {
                params.push(filter.to_query_param());
            }

            if let Some(returning) = self.returning {
                params.push(format!("returning={}", returning));
            }

            if self.upsert {
                // Adjust according to your upsert strategy
                params.push("on_conflict=unique_column".to_string());
            }

            let query = params.join("&");
            (query, Some(self.body))
        }
    }

    pub struct GetQuery {
        pub select_fields: Option<&'static str>,
        pub filters: Vec<filter::Filter>,
        pub ordering: Option<String>,
        pub limits: Option<u64>,
    }

    impl GetQuery {
        pub fn new() -> Self {
            GetQuery {
                select_fields: None,
                filters: Vec::new(),
                ordering: None,
                limits: None,
            }
        }

        pub fn select(mut self, fields: &'static str) -> Self {
            self.select_fields = Some(fields);
            self
        }

        pub fn filter(mut self, condition: filter::Filter) -> Self {
            self.filters.push(condition);
            self
        }

        pub fn order(mut self, field: &str, ascending: bool) -> Self {
            let direction = if ascending { "asc" } else { "desc" };
            self.ordering = Some(format!("order={}.{}", field, direction));
            self
        }

        pub fn limit(mut self, count: u64) -> Self {
            self.limits = Some(count);
            self
        }

        pub fn build(self) -> (String, Option<Vec<u8>>) {
            let mut params = Vec::new();

            if let Some(select) = self.select_fields {
                params.push(format!("select={}", select));
            }

            for filter in self.filters {
                params.push(filter.to_query_param());
            }

            if let Some(ordering) = self.ordering {
                params.push(ordering);
            }

            if let Some(limit) = self.limits {
                params.push(format!("limit={}", limit));
            }

            let query = params.join("&");
            (query, None)
        }
    }

    pub struct PatchQuery {
        pub filters: Vec<filter::Filter>,
        pub returning: Option<&'static str>,
        pub body: Vec<u8>,
    }

    impl PatchQuery {
        pub fn new(body: impl Into<Vec<u8>>) -> Self {
            PatchQuery {
                filters: Vec::new(),
                returning: None,
                body: body.into(),
            }
        }

        pub fn filter(mut self, condition: filter::Filter) -> Self {
            self.filters.push(condition);
            self
        }

        pub fn returning(mut self, fields: &'static str) -> Self {
            self.returning = Some(fields);
            self
        }

        pub fn build(self) -> (String, Option<Vec<u8>>) {
            let mut params = Vec::new();

            for filter in self.filters {
                params.push(filter.to_query_param());
            }

            if let Some(returning) = self.returning {
                params.push(format!("returning={}", returning));
            }

            let query = params.join("&");
            (query, Some(self.body))
        }
    }

    pub struct DeleteQuery {
        pub filters: Vec<filter::Filter>,
        pub returning: Option<&'static str>,
    }

    impl DeleteQuery {
        pub fn new() -> Self {
            DeleteQuery {
                filters: Vec::new(),
                returning: None,
            }
        }

        pub fn filter(mut self, condition: filter::Filter) -> Self {
            self.filters.push(condition);
            self
        }

        pub fn returning(mut self, fields: &'static str) -> Self {
            self.returning = Some(fields);
            self
        }

        pub fn build(self) -> (String, Option<Vec<u8>>) {
            let mut params = Vec::<String>::new();

            for filter in self.filters {
                params.push(filter.to_query_param());
            }

            if let Some(returning) = self.returning {
                params.push(format!("returning={}", returning));
            }

            let query = params.join("&");
            (query, None)
        }
    }

    pub mod filter {
        #[derive(Debug, Clone)]
        pub enum Operator {
            Eq,
            Neq,
            Gt,
            Gte,
            Lt,
            Lte,
            Like,
            Ilike,
            Is,
            In,
        }

        impl Operator {
            fn as_str(&self) -> &'static str {
                match self {
                    Operator::Eq => "eq",
                    Operator::Neq => "neq",
                    Operator::Gt => "gt",
                    Operator::Gte => "gte",
                    Operator::Lt => "lt",
                    Operator::Lte => "lte",
                    Operator::Like => "like",
                    Operator::Ilike => "ilike",
                    Operator::Is => "is",
                    Operator::In => "in",
                }
            }
        }

        #[derive(Debug, Clone)]
        pub struct Filter {
            field: &'static str,
            operator: Operator,
            value: String,
        }

        impl Filter {
            pub fn new<T: ToString>(field: &'static str, operator: Operator, value: T) -> Self {
                Filter {
                    field,
                    operator,
                    value: value.to_string(),
                }
            }

            pub fn to_query_param(&self) -> String {
                format!("{}={}.{}", self.field, self.operator.as_str(), self.value)
            }
        }

        pub struct FilterBuilder {
            field: &'static str,
        }

        impl FilterBuilder {
            pub fn new(field: &'static str) -> Self {
                FilterBuilder { field }
            }

            pub fn eq<T: ToString>(self, value: T) -> Filter {
                Filter::new(&self.field, Operator::Eq, value)
            }

            pub fn neq<T: ToString>(self, value: T) -> Filter {
                Filter::new(&self.field, Operator::Neq, value)
            }

            pub fn gt<T: ToString>(self, value: T) -> Filter {
                Filter::new(&self.field, Operator::Gt, value)
            }

            pub fn gte<T: ToString>(self, value: T) -> Filter {
                Filter::new(&self.field, Operator::Gte, value)
            }

            pub fn lt<T: ToString>(self, value: T) -> Filter {
                Filter::new(&self.field, Operator::Lt, value)
            }

            pub fn lte<T: ToString>(self, value: T) -> Filter {
                Filter::new(&self.field, Operator::Lte, value)
            }

            pub fn like<T: ToString>(self, value: T) -> Filter {
                Filter::new(&self.field, Operator::Like, value)
            }

            pub fn ilike<T: ToString>(self, value: T) -> Filter {
                Filter::new(&self.field, Operator::Ilike, value)
            }

            pub fn is<T: ToString>(self, value: T) -> Filter {
                Filter::new(&self.field, Operator::Is, value)
            }

            pub fn in_list<T: IntoIterator<Item = U>, U: ToString>(self, values: T) -> Filter {
                let value = values
                    .into_iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                Filter::new(self.field, Operator::In, format!("({})", value))
            }
        }
    }
}
