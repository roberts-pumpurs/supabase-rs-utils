use std::borrow::Cow;

use futures::{StreamExt, TryStreamExt};
use supabase_auth::redact::Secret;
use tokio_stream::wrappers::ReceiverStream;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    use supabase_auth::SupabaseAuth;
    let supabase_api_url: url::Url = "http://127.0.0.1:54321".parse().unwrap();
    let supabase_annon_key = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZS1kZW1vIiwicm9sZSI6ImFub24iLCJleHAiOjE5ODM4MTI5OTZ9.CRXP1A7WOeoJeXxjNni43kdQwgnWNReilDMblYTn_I0";
    let supabase_auth =
        SupabaseAuth::new(supabase_api_url.clone(), Cow::Borrowed(&supabase_annon_key));
    let token_refresh = supabase_auth
        .sign_in(supabase_auth::TokenBody {
            email: Cow::Borrowed("worker@swoopscore.com"),
            password: Secret::new(Cow::Borrowed("pass")),
        })
        .unwrap();
    tracing::info!("here");
    let (tx, rx) = tokio::sync::mpsc::channel(5);
    let mut stream = ReceiverStream::new(rx);
    let mut realtime = supabase_realtime::RealtimeConnection::new(supabase_api_url)
        .connect(token_refresh, stream)
        .await
        .unwrap();

    //todo: left off at providing default annon key on the initial connect, then sending
    // heartbeats
    // + updated apikeyso
    // https://supabase.com/docs/guides/realtime/protocol
    // {"topic":"realtime:db","event":"phx_join","payload":{"config":{"broadcast":{"ack":false,"self":false},"presence":{"key":""},"postgres_changes":[{"event":"*","schema":"public","table":"profiles","filter":"id=eq.83a19c16-fcd8-45d0-9710-d7b06ce6f329"}]},"access_token":"eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJhYWwiOiJhYWwxIiwiYW1yIjpbeyJtZXRob2QiOiJwYXNzd29yZCIsInRpbWVzdGFtcCI6MTcyMDU0NzU4Nn1dLCJhcHBfbWV0YWRhdGEiOnsicHJvdmlkZXIiOiJlbWFpbCIsInByb3ZpZGVycyI6WyJlbWFpbCJdfSwiYXVkIjoiYXV0aGVudGljYXRlZCIsImVtYWlsIjoic2NvdXRAc3dvb3BzY29yZS5jb20iLCJleHAiOjE3MjA2MzQ3NjMsImlhdCI6MTcyMDYzNDcwMywiaXNfYW5vbnltb3VzIjpmYWxzZSwiaXNzIjoiaHR0cDovLzEyNy4wLjAuMTo1NDMyMS9hdXRoL3YxIiwicGhvbmUiOiIiLCJyb2xlIjoiYXV0aGVudGljYXRlZCIsInNlc3Npb25faWQiOiJiMGQ5ODY4Mi0zYTEwLTQxOTAtYWZjZC01NWE5Nzc2MGEzZTYiLCJzdWIiOiI4M2ExOWMxNi1mY2Q4LTQ1ZDAtOTcxMC1kN2IwNmNlNmYzMjkiLCJ1c2VyX21ldGFkYXRhIjp7fSwidXNlcl9yb2xlIjoic2NvdXQifQ.Smmu7aH808WzYT3Z82pQGxZQ2NmDsKZL64rG1uJ_wtQ"},"ref":"1","join_ref":"1"}
    // {"topic":"realtime:db","event":"phx_leave","payload":{},"ref":"2","join_ref":"1"}
    // {"topic":"realtime:db","event":"phx_join","payload":{"config":{"broadcast":{"ack":false,"self":false},"presence":{"key":""},"postgres_changes":[{"event":"*","schema":"public","table":"assignments"},{"event":"UPDATE","schema":"public","table":"rooms"}]},"access_token":"eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJhYWwiOiJhYWwxIiwiYW1yIjpbeyJtZXRob2QiOiJwYXNzd29yZCIsInRpbWVzdGFtcCI6MTcyMDU0NzU4Nn1dLCJhcHBfbWV0YWRhdGEiOnsicHJvdmlkZXIiOiJlbWFpbCIsInByb3ZpZGVycyI6WyJlbWFpbCJdfSwiYXVkIjoiYXV0aGVudGljYXRlZCIsImVtYWlsIjoic2NvdXRAc3dvb3BzY29yZS5jb20iLCJleHAiOjE3MjA2MzQ3NjMsImlhdCI6MTcyMDYzNDcwMywiaXNfYW5vbnltb3VzIjpmYWxzZSwiaXNzIjoiaHR0cDovLzEyNy4wLjAuMTo1NDMyMS9hdXRoL3YxIiwicGhvbmUiOiIiLCJyb2xlIjoiYXV0aGVudGljYXRlZCIsInNlc3Npb25faWQiOiJiMGQ5ODY4Mi0zYTEwLTQxOTAtYWZjZC01NWE5Nzc2MGEzZTYiLCJzdWIiOiI4M2ExOWMxNi1mY2Q4LTQ1ZDAtOTcxMC1kN2IwNmNlNmYzMjkiLCJ1c2VyX21ldGFkYXRhIjp7fSwidXNlcl9yb2xlIjoic2NvdXQifQ.Smmu7aH808WzYT3Z82pQGxZQ2NmDsKZL64rG1uJ_wtQ"},"ref":"3","join_ref":"3"}
    // while let Some(msg) = realtime.next().await {
    //     tracing::debug!(?msg, "reading frame");
    // }
    // let msg = match ws.read_frame().await {
    //     Ok(msg) => msg,
    //     Err(e) => {
    //         tracing::error!("Error: {}", e);
    //         ws.write_frame(Frame::close_raw(vec![].into()))
    //             .await
    //             .unwrap();
    //         break;
    //     }
    // };
    // tracing::info!(?msg.opcode, "msg");
    // match msg.opcode {
    //     OpCode::Text => {
    //         let payload = String::from_utf8(msg.payload.to_vec()).expect("Invalid UTF-8
    // data");         // Normally deserialise from json here, print just to show it
    // works         tracing::info!("{:?}", payload);
    //     }
    //     OpCode::Close => {
    //         break;
    //     }
    //     _ => {}
    // }
    panic!()
}
