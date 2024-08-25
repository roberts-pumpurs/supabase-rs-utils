use std::borrow::{BorrowMut, Cow};
use std::future::Future;
use std::ops::DerefMut;
use std::task::{ready, Poll, Waker};

use connection::WsSupabaseConnection;
use fastwebsockets::{
    FragmentCollector, FragmentCollectorRead, Frame, WebSocketError, WebSocketWrite,
};
use futures::{pin_mut, AsyncWrite, FutureExt, Sink, SinkExt, Stream, StreamExt, TryFuture};
use hyper::upgrade::Upgraded;
// use hyper_util::rt::,
use message::InboundProtocolMesseage;
use pin_project::pin_project;
use supabase_auth::AuthResponse;
use tokio::io::WriteHalf;
use tracing::instrument::WithSubscriber;

mod connection;
mod error;
pub mod message;

pub struct RealtimeConnection {
    url: url::Url,
}

impl RealtimeConnection {
    pub fn new(url: url::Url) -> Self {
        Self { url }
    }

    pub async fn connect<'a, S: Stream<Item = message::InboundProtocolMesseage> + Unpin>(
        self,
        jwt_stream: supabase_auth::RefreshStream<'a, 'a>,
        input_stream: S,
    ) -> Result<LiveRealtimeConnection<S>, error::SupabaseRealtimeError> {
        let supabase_annon_key = jwt_stream.api_key.as_ref();
        let url = self.url.join(
            format!("realtime/v1/websocket?apikey={supabase_annon_key}&vsn=1.0.0").as_str(),
        )?;
        let mut con = connection::connect(&url).await?;
        let (mut from_ws_sender, from_ws_receiver) =
            tokio::sync::mpsc::channel::<serde_json::Value>(10);
        let (to_ws_sender, mut to_ws_reader) = tokio::sync::mpsc::channel::<serde_json::Value>(10);
        let (waker_sender, waker_receiver) = tokio::sync::oneshot::channel::<Waker>();

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
                            read_loop(item, &mut from_ws_sender).await;
                            waker.wake_by_ref();
                        } else {
                            tracing::error!("ws socket exited");
                            break;
                        }
                    }
                    item = to_ws_reader.recv() =>{
                        if let Some(item) = item {
                            // write to ws
                            write_loop(item, con).await;
                        } else {
                            tracing::error!("ws reader channel exited");
                            break;
                        }
                    }
                }
            }
        });

        let res = LiveRealtimeConnection {
            handle,
            to_ws_sender,
            from_ws_receiver,
            jwt_stream,
            input_stream,
            state: RealtimeConnectionState::ReadJwt,
            auth_response: None,
            message_to_send: None,
            oneshot: Some(waker_sender),
        };

        Ok(res)
    }
}

async fn write_loop(item: serde_json::Value, con: &mut WsSupabaseConnection) {
    let message_bytes = serde_json::to_vec(&item).unwrap();
    let payload = fastwebsockets::Payload::<'static>::Owned(message_bytes);
    let frame = Frame::<'static>::text(payload);
    con.write_frame(frame).await.unwrap();
}

async fn read_loop(
    item: fastwebsockets::Frame<'_>,
    from_ws_sender: &mut tokio::sync::mpsc::Sender<serde_json::Value>,
) {
    let item = serde_json::from_slice(&item.payload).unwrap();
    from_ws_sender.send(item).await.unwrap();
}

#[pin_project]
pub struct LiveRealtimeConnection<
    'a,
    T: Stream<Item = message::InboundProtocolMesseage> + std::marker::Unpin,
> {
    to_ws_sender: tokio::sync::mpsc::Sender<serde_json::Value>,
    from_ws_receiver: tokio::sync::mpsc::Receiver<serde_json::Value>,
    handle: tokio::task::JoinHandle<()>,
    oneshot: Option<tokio::sync::oneshot::Sender<Waker>>,
    #[pin]
    jwt_stream: supabase_auth::RefreshStream<'a, 'a>,
    #[pin]
    state: RealtimeConnectionState,
    #[pin]
    input_stream: T,
    #[pin]
    auth_response: Option<AuthResponse>,
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
    T: Stream<Item = message::InboundProtocolMesseage> + std::marker::Unpin,
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
                        Poll::Ready(Some(Ok(jwt))) => {
                            this.auth_response.set(Some(jwt.auth_data));
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
                            Poll::Ready(Some(msg)) => {
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
