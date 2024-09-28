mod authenticated;
mod unauthenticated;

use std::future::Future;
use std::marker::PhantomData;

pub use authenticated::AuthenticatedSupabaseClient;
use reqwest::header;
use supabase_auth::SUPABASE_KEY;
use tracing::{info_span, Instrument};
pub use unauthenticated::SupabaseClient;

use crate::error;
use crate::error::SupabaseClientError;

pub trait SupabaseClientExt {
    fn client(&mut self) -> reqwest::Client;
    fn supabase_url(&self) -> &url::Url;

    fn build_request<T: PostgRestQuery>(
        &mut self,
        query: T,
    ) -> Result<SupabaseRequest<T>, SupabaseClientError> {
        let query_builder = query.to_query()?;
        let method = query_builder.reqwest_method();
        let client = self.client();
        let (path, body) = query_builder.build();
        let url = self.supabase_url().join("/rest/v1/")?.join(path.as_str())?;
        let request = client.request(method, url.as_str());
        let request = if let Some(body) = body {
            request.body(body)
        } else {
            request
        }
        .build()?;

        Ok(SupabaseRequest {
            request,
            client,
            query: PhantomData,
        })
    }
}

struct SupabaseRequest<T: PostgRestQuery> {
    request: reqwest::Request,
    client: reqwest::Client,
    query: PhantomData<T>,
}

impl<T: PostgRestQuery> SupabaseRequest<T> {
    pub async fn execute(self) -> Result<SupabaseResponse<T>, SupabaseClientError> {
        let response = self.client.execute(self.request).await?;

        Ok(SupabaseResponse {
            response,
            query: PhantomData,
        })
    }
}

struct SupabaseResponse<T: PostgRestQuery> {
    response: reqwest::Response,
    query: PhantomData<T>,
}
impl<T: PostgRestQuery> SupabaseResponse<T> {
    pub async fn ok(self) -> Result<(), SupabaseClientError> {
        self.response.error_for_status()?;
        Ok(())
    }

    pub async fn json_err(
        self,
    ) -> Result<Result<(), error::postgrest_error::Error>, SupabaseClientError> {
        let status = self.response.status();
        let mut bytes = self.response.bytes().await?.to_vec();
        if status.is_success() {
            Ok(Ok(()))
        } else {
            {
                let json = String::from_utf8_lossy(bytes.as_ref());
                tracing::error!(
                    status = %status,
                    body = %json,
                    "Failed to execute request"
                );
            };

            let error =
                simd_json::from_slice::<error::postgrest_error::ErrorResponse>(bytes.as_mut())?;
            let error = error::postgrest_error::Error::from_error_response(error);
            Ok(Err(error))
        }
    }

    pub async fn json(
        self,
    ) -> Result<Result<T::Output, error::postgrest_error::Error>, SupabaseClientError> {
        let status = self.response.status();
        let mut bytes = self.response.bytes().await?.to_vec();
        if status.is_success() {
            {
                let json = String::from_utf8_lossy(bytes.as_ref());
                tracing::info!(response_body = ?json, "Response JSON");
            };
            let result = simd_json::from_slice::<T::Output>(bytes.as_mut())?;
            Ok(Ok(result))
        } else {
            {
                let json = String::from_utf8_lossy(bytes.as_ref());
                tracing::error!(
                    status = %status,
                    body = %json,
                    "Failed to execute request"
                );
            };

            let error =
                simd_json::from_slice::<error::postgrest_error::ErrorResponse>(bytes.as_mut())?;
            let error = error::postgrest_error::Error::from_error_response(error);
            Ok(Err(error))
        }
    }
}

impl SupabaseClientExt for AuthenticatedSupabaseClient {
    fn client(&mut self) -> reqwest::Client {
        let client = self.client.read().expect("rw lock is poisoned");
        client.clone()
    }

    fn supabase_url(&self) -> &url::Url {
        &self.supabase_url
    }
}

impl SupabaseClientExt for SupabaseClient {
    fn client(&mut self) -> reqwest::Client {
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

pub mod query_builder {

    #[derive(Debug, Clone, PartialEq, PartialOrd)]
    pub struct QueryBuilder {
        pub table_name: &'static str,
        pub operation: QueryBuilderOperation,
    }

    #[derive(Debug, Clone, PartialEq, PartialOrd)]
    pub enum QueryBuilderOperation {
        Post(PostQuery),
        Get(GetQuery),
        Patch(PatchQuery),
        Delete(DeleteQuery),
    }

    impl QueryBuilder {
        pub fn build(self) -> (String, Option<Vec<u8>>) {
            let (query, body) = match self.operation {
                QueryBuilderOperation::Post(query) => query.build(),
                QueryBuilderOperation::Get(query) => query.build(),
                QueryBuilderOperation::Patch(query) => query.build(),
                QueryBuilderOperation::Delete(query) => query.build(),
            };
            (format!("{}?{query}", self.table_name), body)
        }

        pub fn reqwest_method(&self) -> reqwest::Method {
            match self.operation {
                QueryBuilderOperation::Post(_) => reqwest::Method::POST,
                QueryBuilderOperation::Get(_) => reqwest::Method::GET,
                QueryBuilderOperation::Patch(_) => reqwest::Method::PATCH,
                QueryBuilderOperation::Delete(_) => reqwest::Method::DELETE,
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, PartialOrd)]
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

    #[derive(Debug, Clone, PartialEq, PartialOrd)]
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

    #[derive(Debug, Clone, PartialEq, PartialOrd)]
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

    #[derive(Debug, Clone, PartialEq, PartialOrd)]
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
        #[derive(Debug, Clone, PartialEq, PartialOrd)]
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

        #[derive(Debug, Clone, PartialEq, PartialOrd)]
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

        #[derive(Debug, Clone, PartialEq, PartialOrd)]
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
