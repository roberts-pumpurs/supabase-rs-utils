use std::task::{Poll, Waker};

use connection::WsSupabaseConnection;
use fastwebsockets::Frame;
use futures::{Stream, StreamExt};
use pin_project::pin_project;
use supabase_auth::AuthResponse;

mod connection;
mod error;
pub mod message;

pub struct RealtimeConnection {
    url: url::Url,
}

impl RealtimeConnection {
    const WS_RECV_BUFFER: usize = 10;
    const WS_SEND_BUFFER: usize = 10;

    pub fn new(url: url::Url) -> Self {
        Self { url }
    }

    pub async fn connect<'a, S: Stream<Item = message::ProtocolMesseage> + Unpin>(
        self,
        mut jwt_stream: supabase_auth::RefreshStream<'a>,
        input_stream: S,
    ) -> Result<LiveRealtimeConnection<S>, error::SupabaseRealtimeError> {
        let supabase_annon_key = &jwt_stream.api_key;
        let url = self.url.join(
            format!("realtime/v1/websocket?apikey={supabase_annon_key}&vsn=1.0.0").as_str(),
        )?;
        let mut con = connection::connect(&url).await?;
        let initial_auth_response = jwt_stream
            .next()
            .await
            .ok_or(error::SupabaseRealtimeError::JwtStreamClosedUnexpectedly)??;
        let (mut from_ws_sender, from_ws_receiver) =
            tokio::sync::mpsc::channel::<serde_json::Value>(Self::WS_RECV_BUFFER);
        let (to_ws_sender, mut to_ws_reader) =
            tokio::sync::mpsc::channel::<serde_json::Value>(Self::WS_SEND_BUFFER);
        let (waker_sender, waker_receiver) = tokio::sync::oneshot::channel::<Waker>();
        // todo: get the Waker object here rather than using oneshot channel for sending it
        // tood: add periodic sending of heartbeats

        let handle = tokio::spawn(async move {
            let con = &mut con;
            let Ok(waker) = waker_receiver.await else {
                tracing::error!("waker dropped");
                return;
            };
            tracing::trace!("waker received");
            loop {
                tokio::select! {
                    item = con.read_frame() => {
                        if let Ok(item) = item {
                            // read from ws and send to the async task
                            if read_loop(item, &mut from_ws_sender).await.is_err() {
                                break;
                            }
                            waker.wake_by_ref();
                        } else {
                            tracing::error!("ws socket exited");
                            break;
                        }
                    }
                    item = to_ws_reader.recv() =>{
                        if let Some(item) = item {
                            // write to ws
                            if write_loop(item, con).await.is_err() {
                                break;
                            }
                        } else {
                            tracing::error!("ws reader channel exited");
                            break;
                        }
                    }
                }
            }
            waker.wake_by_ref();
        });

        let res = LiveRealtimeConnection {
            handle,
            to_ws_sender,
            from_ws_receiver,
            jwt_stream,
            input_stream,
            state: RealtimeConnectionState::ReadJwt,
            auth_response: initial_auth_response,
            message_to_send: None,
            oneshot: Some(waker_sender),
        };

        Ok(res)
    }
}

async fn write_loop(item: serde_json::Value, con: &mut WsSupabaseConnection) -> eyre::Result<()> {
    if let Ok(message_bytes) = serde_json::to_vec(&item) {
        let payload = fastwebsockets::Payload::<'static>::Owned(message_bytes);
        let frame = Frame::<'static>::text(payload);
        con.write_frame(frame).await?;
    } else {
        tracing::error!("could not vectorise json data");
    }
    Ok(())
}

async fn read_loop(
    item: fastwebsockets::Frame<'_>,
    from_ws_sender: &mut tokio::sync::mpsc::Sender<serde_json::Value>,
) -> eyre::Result<()> {
    let from_slice = serde_json::from_slice(&item.payload);
    match from_slice {
        Ok(item) => {
            from_ws_sender.send(item).await?;
        }
        Err(err) => {
            tracing::error!(?err, payload =? &item.payload, "erorr when deserialising data");
        }
    };
    Ok(())
}

#[pin_project]
pub struct LiveRealtimeConnection<
    'a,
    T: Stream<Item = message::ProtocolMesseage> + std::marker::Unpin,
> {
    to_ws_sender: tokio::sync::mpsc::Sender<serde_json::Value>,
    from_ws_receiver: tokio::sync::mpsc::Receiver<serde_json::Value>,
    handle: tokio::task::JoinHandle<()>,
    oneshot: Option<tokio::sync::oneshot::Sender<Waker>>,
    #[pin]
    jwt_stream: supabase_auth::RefreshStream<'a>,
    #[pin]
    state: RealtimeConnectionState,
    #[pin]
    input_stream: T,
    #[pin]
    auth_response: AuthResponse,
    #[pin]
    message_to_send: Option<serde_json::Value>,
}

#[derive(Debug)]
enum RealtimeConnectionState {
    ReadJwt,
    ReadInputMessage,
    SendInputMessage,
    ReadWsMessage,
}

impl<'a, T> Stream for LiveRealtimeConnection<'a, T>
where
    T: Stream<Item = message::ProtocolMesseage> + std::marker::Unpin,
{
    type Item = Result<serde_json::Value, error::SupabaseRealtimeError>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let mut this = self.project();
        if let Some(oneshot) = this.oneshot.take() {
            if let Err(_) = oneshot.send(cx.waker().clone()) {
                return Poll::Ready(None)
            }
        }
        loop {
            let state = this.state.as_mut().get_mut();
            tracing::trace!(state =? state, "trace");
            match state {
                RealtimeConnectionState::ReadJwt => {
                    let jwt = this.jwt_stream.poll_next_unpin(cx);

                    match jwt {
                        Poll::Ready(Some(Ok(auth_resp))) => {
                            tracing::debug!("new jwt set");
                            this.auth_response.set(auth_resp);
                        }
                        Poll::Ready(Some(Err(err))) => {
                            tracing::warn!(?err, "refresh stream error");
                            return Poll::Pending;
                        }
                        Poll::Ready(None) => return Poll::Ready(None),
                        Poll::Pending => (),
                    }

                    this.state.set(RealtimeConnectionState::ReadInputMessage);
                }
                RealtimeConnectionState::ReadInputMessage => {
                    if this.message_to_send.is_none() {
                        let msg = this.input_stream.poll_next_unpin(cx);
                        match msg {
                            Poll::Ready(Some(mut msg)) => {
                                let resp = this.auth_response.as_ref().get_ref();
                                let AuthResponse { access_token, .. } = resp;
                                msg.set_access_token(&access_token);

                                let msg = serde_json::to_value(&msg)
                                    .expect("could not serialize inbound message");
                                this.message_to_send.set(Some(msg));
                                this.state.set(RealtimeConnectionState::SendInputMessage);
                                continue;
                            }
                            Poll::Ready(None) => return Poll::Ready(None),
                            Poll::Pending => (),
                        }
                    }
                    this.state.set(RealtimeConnectionState::ReadWsMessage);
                }
                RealtimeConnectionState::SendInputMessage => {
                    if let Some(msg) = this.message_to_send.take() {
                        use tokio::sync::mpsc::error::TrySendError;
                        match this.to_ws_sender.try_send(msg) {
                            Ok(()) => {}
                            Err(TrySendError::Full(msg)) => {
                                tracing::warn!("could not send message");
                                this.message_to_send.set(Some(msg));
                            }
                            Err(TrySendError::Closed(_)) => {
                                tracing::error!("ws sender channel has been closed");
                                return Poll::Ready(None);
                            }
                        }
                        cx.waker().wake_by_ref();
                    }
                    this.state.set(RealtimeConnectionState::ReadWsMessage);
                }
                RealtimeConnectionState::ReadWsMessage => {
                    if let Ok(msg) = this.from_ws_receiver.try_recv() {
                        return Poll::Ready(Some(Ok(msg)))
                    }
                    break
                }
            }
        }

        // set the new default state
        this.state.set(RealtimeConnectionState::ReadJwt);
        if this.handle.is_finished() {
            tracing::error!("the tokio handle has closed");
            return Poll::Ready(None)
        }
        return Poll::Pending;
    }
}
