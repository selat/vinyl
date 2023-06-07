use ethers::{
    abi::RawLog,
    contract::EthEvent,
    prelude::abigen,
    providers::{Provider, Ws},
    types::Log,
};
use log::info;
use serde::ser::{Serialize, SerializeStruct, Serializer};
mod vinyl;

abigen!(UniswapV2Pair, "src/UniswapV2Pair.json");

impl Serialize for SyncFilter {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("Sync", 2)?;
        s.serialize_field("reserve0", &self.reserve_0)?;
        s.serialize_field("reserve1", &self.reserve_1)?;
        s.end()
    }
}

struct MyHandler {}

impl vinyl::EventHandler for MyHandler {
    fn handle(&self, log: &Log) {
        let address = log.address;
        let raw_log = RawLog::from(log.clone());
        let sync_event = SyncFilter::decode_log(&raw_log).unwrap();
        info!(
            "Decoded log from {:?}: {}",
            address,
            serde_json::to_string(&sync_event).unwrap()
        );
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    env_logger::builder().format_timestamp_micros().init();

    let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is not set");
    let provider = Provider::<Ws>::connect(rpc_url).await?;

    let mut vinyl_sync = vinyl::VinylSync::new(&provider);
    vinyl_sync.add_handler(&SyncFilter::abi_signature(), Box::new(MyHandler {}));
    vinyl_sync.run().await?;

    Ok(())
}
