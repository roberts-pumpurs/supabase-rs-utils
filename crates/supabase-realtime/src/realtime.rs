use alloc::sync::Arc;
use core::task::Poll;

use fastwebsockets::Frame;
use futures::stream::FuturesUnordered;
use futures::{SinkExt as _, Stream, StreamExt as _};
use rp_supabase_auth::types::LoginCredentials;
use tokio::sync::Mutex;
use tokio::time::timeout;
use tokio_stream::wrappers::IntervalStream;

use crate::connection::WsSupabaseConnection;
use crate::error::SupabaseRealtimeError;
use crate::message::access_token::AccessToken;
use crate::message::{broadcast, phx_join, ProtocolMessage, ProtocolPayload};
use crate::{connection, error, message};

pub struct RealtimeConnectionClient {
    tx: futures::channel::mpsc::UnboundedSender<ProtocolPayload>,
}

impl RealtimeConnectionClient {
    pub async fn subscribe_to_changes(
        &mut self,
        join: phx_join::PhxJoin,
    ) -> Result<(), futures::channel::mpsc::SendError> {
        self.tx.send(ProtocolPayload::PhxJoin(join)).await
    }

    pub async fn broadcast(
        &mut self,
        msg: broadcast::Broadcast,
    ) -> Result<(), futures::channel::mpsc::SendError> {
        self.tx.send(ProtocolPayload::Broadcast(msg)).await
    }
}

pub struct RealtimeConnection {
    topic: String,
    config: rp_supabase_auth::jwt_stream::SupabaseAuthConfig,
}

type RealtimeStreamType = Result<ProtocolMessage, SupabaseRealtimeError>;

impl RealtimeConnection {
    const HEARTBEAT_PERIOD: core::time::Duration = core::time::Duration::from_secs(20);

    #[must_use]
    pub fn new_db_updates(config: rp_supabase_auth::jwt_stream::SupabaseAuthConfig) -> Self {
        const DB_UPDATE_TOPIC: &str = "table-db-changes";
        Self::new(config, DB_UPDATE_TOPIC)
    }

    #[must_use]
    pub fn new(config: rp_supabase_auth::jwt_stream::SupabaseAuthConfig, topic: &str) -> Self {
        let prefix = "realtime";
        let topic = [prefix, topic].join(":");
        Self { topic, config }
    }

    #[tracing::instrument(skip_all, err)]
    pub async fn connect(
        self,
        login_info: LoginCredentials,
    ) -> Result<
        (
            impl Stream<Item = RealtimeStreamType>,
            RealtimeConnectionClient,
        ),
        SupabaseRealtimeError,
    > {
        let supabase_annon_key = &self.config.api_key;
        let realtime_url = self.config.url.join(
            format!("realtime/v1/websocket?apikey={supabase_annon_key}&vsn=1.0.0").as_str(),
        )?;

        let mut auth_stream = rp_supabase_auth::jwt_stream::JwtStream::new(self.config.clone())
            .sign_in(login_info)?;
        let mut latest_access_token = loop {
            match auth_stream.next().await {
                Some(Ok(new_latest_access_token)) => {
                    let Some(access_token) = new_latest_access_token.access_token else {
                        tracing::error!("access token was not present!");
                        continue;
                    };
                    break access_token;
                }
                Some(Err(err)) => {
                    tracing::error!(?err, "initial jwt fetch err");
                }
                None => return Err(error::SupabaseRealtimeError::JwtStreamClosedUnexpectedly),
            }
        };

        let mut ref_counter = 0_u64;
        let mut join_ref_counter = 0;
        let (tx, rx) = futures::channel::mpsc::unbounded();
        let topic = self.topic.clone();
        let input_stream = rx
            .map(move |item| {
                ref_counter += 1;
                join_ref_counter += 1;
                message::ProtocolMessage {
                    topic: topic.clone(),
                    payload: item,
                    ref_field: Some(ref_counter.to_string()),
                    join_ref: Some(join_ref_counter.to_string()),
                }
            })
            .map(Ok)
            .boxed();

        let heartbeat_stream = {
            let mut interval = tokio::time::interval(Self::HEARTBEAT_PERIOD);
            interval.reset();
            let interval_stream = IntervalStream::new(interval).fuse();
            interval_stream
                .map(move |_s| message::ProtocolMessage {
                    topic: "phoenix".to_owned(),
                    payload: message::ProtocolPayload::Heartbeat(message::heartbeat::Heartbeat),
                    ref_field: None,
                    join_ref: None,
                })
                .map(Ok)
                .boxed()
        };

        let topic = self.topic.clone();
        let access_token_stream = {
            auth_stream
                .filter_map(move |item| {
                    let topic = topic.clone();
                    async move {
                        item.map(|item| {
                            if let Some(access_token) = item.access_token {
                                return Some(message::ProtocolMessage {
                                    topic: topic.clone(),
                                    payload: message::ProtocolPayload::AccessToken(AccessToken {
                                        access_token,
                                    }),
                                    ref_field: None,
                                    join_ref: None,
                                })
                            }
                            None
                        })
                        .map_err(SupabaseRealtimeError::from)
                        .transpose()
                    }
                })
                .boxed()
        };
        let input_stream =
            futures::stream::select_all([input_stream, heartbeat_stream, access_token_stream])
                .map(move |mut item| {
                    if let Ok(item) = &mut item {
                        if let message::ProtocolPayload::AccessToken(at) = &mut item.payload {
                            latest_access_token = at.access_token.clone();
                        }
                        item.set_access_token(&latest_access_token);
                    }
                    item
                })
                .map(move |mut item| {
                    ref_counter += 1;
                    if let Ok(item) = &mut item {
                        item.ref_field = Some(ref_counter.to_string());
                    }
                    item
                });

        let client = RealtimeConnectionClient { tx };
        let output_stream = RealtimeBaseConnection::new(realtime_url)
            .connect(input_stream)
            .await?;
        Ok((output_stream, client))
    }
}

pub struct RealtimeBaseConnection {
    url: url::Url,
}

impl RealtimeBaseConnection {
    #[must_use]
    pub const fn new(url: url::Url) -> Self {
        Self { url }
    }
    pub async fn connect<S: Stream<Item = RealtimeStreamType> + Unpin>(
        self,
        mut input_stream: S,
    ) -> Result<impl Stream<Item = RealtimeStreamType>, error::SupabaseRealtimeError> {
        tracing::info!(url =? self.url.as_str(), "Starting RealtimeConnection::connect");

        let con = Arc::new(Mutex::new(connection::connect(&self.url).await?));
        tracing::info!("WebSocket connection established");

        let mut write_futures = FuturesUnordered::new();
        let mut reat_future = FuturesUnordered::new();
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        let read_task = {
            let con = Arc::clone(&con);
            async move {
                let con = Arc::clone(&con);
                read_from_ws(&con, tx).await;
            }
        };
        reat_future.push(read_task);

        let stream_to_return = futures::stream::poll_fn(move |cx| {
            match input_stream.poll_next_unpin(cx) {
                Poll::Ready(Some(message_to_send)) => {
                    let con = Arc::clone(&con);
                    match message_to_send {
                        Ok(message) => {
                            write_futures.push(async move {
                                let con = Arc::clone(&con);
                                send(message, &con).await
                            });
                        }
                        Err(err) => {
                            cx.waker().wake_by_ref();
                            return Poll::Ready(Some(Err(err)));
                        }
                    }
                }
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => {}
            }

            match reat_future.poll_next_unpin(cx) {
                Poll::Ready(_) => {
                    tracing::info!("Read task completed");
                    return Poll::Ready(None);
                }
                Poll::Pending => {}
            };

            match write_futures.poll_next_unpin(cx) {
                Poll::Ready(Some(res)) => match res {
                    Ok(()) => {
                        tracing::debug!("Message sent successfully");
                    }
                    Err(err) => {
                        tracing::error!(?err, "Error sending message");
                        cx.waker().wake_by_ref();
                        return Poll::Ready(Some(Err(err)));
                    }
                },
                Poll::Ready(None) => {}
                Poll::Pending => {}
            };

            match rx.poll_next_unpin(cx) {
                Poll::Ready(Some(item)) => {
                    tracing::debug!(?item, "Received item");
                    cx.waker().wake_by_ref();
                    Poll::Ready(Some(Ok(item)))
                }
                Poll::Ready(None) => Poll::Ready(None),
                Poll::Pending => Poll::Pending,
            }
        });
        Ok(stream_to_return)
    }
}

async fn read_from_ws(
    con: &Mutex<WsSupabaseConnection>,
    mut tx: futures::channel::mpsc::UnboundedSender<ProtocolMessage>,
) {
    tracing::info!("Starting read_from_ws task");
    let duration = core::time::Duration::from_millis(100);
    loop {
        let mut con = con.lock().await;
        let Ok(frame) = timeout(duration, con.read_frame()).await else {
            continue;
        };
        drop(con);

        let mut frame = match frame {
            Ok(frame) => frame,
            Err(err) => {
                tracing::error!(?err, "Error reading frame");
                continue;
            }
        };
        let repr = String::from_utf8_lossy(&frame.payload);
        tracing::debug!(?repr, "Received frame");

        let from_slice = simd_json::from_slice(frame.payload.to_mut());
        match from_slice {
            Ok(item) => {
                let _res = tx.send(item).await;
            }
            Err(err) => {
                let repr = String::from_utf8_lossy(&frame.payload);
                tracing::error!(?err, payload = ?repr, "Error deserializing data");
            }
        };
    }
}

async fn send(
    message_to_send: ProtocolMessage,
    con: &Mutex<WsSupabaseConnection>,
) -> Result<(), error::SupabaseRealtimeError> {
    tracing::debug!(?message_to_send, "Sending message");
    let message_bytes = simd_json::to_vec(&message_to_send)?;
    let payload = fastwebsockets::Payload::<'static>::Owned(message_bytes);
    let frame = Frame::<'static>::text(payload);
    let mut con = con.lock().await;
    con.write_frame(frame).await?;
    drop(con);
    Ok(())
}
