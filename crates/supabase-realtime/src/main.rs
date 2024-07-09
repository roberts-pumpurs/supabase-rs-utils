use connection::connect;
use fastwebsockets::{Frame, OpCode};

mod connection;
mod error;

#[tokio::main(flavor = "current_thread")]
async fn main() -> eyre::Result<()> {
    let domain = "wss://data-stream.binance.com:9443/ws/btcusdt@bookTicker".parse()?;
    let mut ws = connect(&domain).await?;

    loop {
        let msg = match ws.read_frame().await {
            Ok(msg) => msg,
            Err(e) => {
                println!("Error: {}", e);
                ws.write_frame(Frame::close_raw(vec![].into())).await?;
                break;
            }
        };

        match msg.opcode {
            OpCode::Text => {
                let payload = String::from_utf8(msg.payload.to_vec()).expect("Invalid UTF-8 data");
                // Normally deserialise from json here, print just to show it works
                println!("{:?}", payload);
            }
            OpCode::Close => {
                break;
            }
            _ => {}
        }
    }
    Ok(())
}
