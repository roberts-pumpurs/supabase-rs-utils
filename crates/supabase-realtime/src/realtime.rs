use std::rc::Rc;
use std::task::Poll;

use fastwebsockets::Frame;
use futures::stream::FuturesUnordered;
use futures::{future, pin_mut, FutureExt, SinkExt, Stream, StreamExt};
use supabase_auth::AuthResponse;
use tokio::sync::{Mutex, RwLock};
use tokio::time::timeout;
use tokio_stream::wrappers::IntervalStream;

use crate::connection::WsSupabaseConnection;
use crate::message::ProtocolMessage;
use crate::{connection, error, message};

pub struct RealtimeConnection {
    url: url::Url,
}

impl RealtimeConnection {
    const HEARTBEAT_PERIOD: std::time::Duration = std::time::Duration::from_secs(20);

    pub fn new(url: url::Url) -> Self {
        Self { url }
    }
    pub async fn connect<'a, S: Stream<Item = message::ProtocolMessage> + Unpin>(
        self,
        mut jwt_stream: supabase_auth::RefreshStream,
        mut input_stream: S,
    ) -> Result<
        impl Stream<Item = Result<message::ProtocolMessage, error::SupabaseRealtimeError>>,
        error::SupabaseRealtimeError,
    > {
        tracing::info!("Starting RealtimeConnection::connect");
        let supabase_annon_key = &jwt_stream.api_key;
        let url = self.url.join(
            format!("realtime/v1/websocket?apikey={supabase_annon_key}&vsn=1.0.0").as_str(),
        )?;
        tracing::debug!(?url, "WebSocket URL constructed");

        let mut interval = tokio::time::interval(Self::HEARTBEAT_PERIOD);
        interval.reset();
        let mut interval_stream = IntervalStream::new(interval).fuse();

        let con = Rc::new(Mutex::new(connection::connect(&url).await?));
        tracing::info!("WebSocket connection established");

        let mut write_futures = FuturesUnordered::new();
        let mut reat_future = FuturesUnordered::new();
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        let read_task = {
            let con = Rc::clone(&con);
            read_from_ws(con, tx)
        };
        reat_future.push(read_task);

        let mut latest_access_token = loop {
            match jwt_stream.next().await {
                Some(Ok(new_latest_access_token)) => {
                    break new_latest_access_token.access_token;
                }
                Some(Err(err)) => {
                    tracing::error!(?err, "initial jwt fetch err");
                }
                None => return Err(error::SupabaseRealtimeError::JwtStreamClosedUnexpectedly),
            }
        };

        let stream_to_return = futures::stream::poll_fn(move |cx| {
            match jwt_stream.poll_next_unpin(cx) {
                Poll::Ready(Some(Ok(new_auth_resp))) => {
                    latest_access_token = new_auth_resp.access_token;
                    let msg = message::ProtocolMessage::AccessToken(message::PhoenixMessage {
                        topic: "realtime:table-db-changes".to_string(),
                        payload: message::access_token::AccessToken {
                            access_token: latest_access_token.clone(),
                        },
                        ref_field: None,
                        join_ref: None,
                    });
                    let con = Rc::clone(&con);
                    write_futures.push(send(msg, con));
                    tracing::debug!("Received new access token");
                }
                Poll::Ready(Some(Err(jwt_error))) => {
                    tracing::error!(?jwt_error, "JWT stream error");
                    cx.waker().wake_by_ref();
                    return Poll::Ready(Some(Err(
                        error::SupabaseRealtimeError::RefreshStreamError(jwt_error),
                    )));
                }
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => {}
            };
            match interval_stream.poll_next_unpin(cx) {
                Poll::Ready(Some(_instant)) => {
                    tracing::debug!("Sending heartbeat message");
                    let mut hb_msg = message::ProtocolMessage::Heartbeat(message::PhoenixMessage {
                        topic: "phoenix".to_string(),
                        payload: message::heartbeat::Heartbeat,
                        ref_field: None,
                        join_ref: None,
                    });
                    let con = Rc::clone(&con);
                    hb_msg.set_access_token(&latest_access_token);
                    write_futures.push(send(hb_msg, con));
                }
                Poll::Ready(None) => {
                    return Poll::Ready(None);
                }
                Poll::Pending => {}
            }

            match input_stream.poll_next_unpin(cx) {
                Poll::Ready(Some(mut message_to_send)) => {
                    message_to_send.set_access_token(&latest_access_token);
                    let con = Rc::clone(&con);
                    write_futures.push(send(message_to_send, con));
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
                    Ok(_) => {
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
                    return Poll::Ready(Some(Ok(item)));
                }
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
            }
        });
        Ok(stream_to_return)
    }
}

async fn read_from_ws(
    con: Rc<Mutex<WsSupabaseConnection>>,
    mut tx: futures::channel::mpsc::UnboundedSender<ProtocolMessage>,
) {
    tracing::info!("Starting read_from_ws task");
    let duration = std::time::Duration::from_millis(100);
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
    con: Rc<Mutex<WsSupabaseConnection>>,
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
