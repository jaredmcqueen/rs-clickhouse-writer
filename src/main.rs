use clickhouse::inserter::Quantities;
use serde::Serialize;
use std::fmt;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use stock::stock_data::Data::Bar;
use stock::stock_data::Data::DailyBar;
use stock::stock_data::Data::Luld;
use stock::stock_data::Data::Quote;
use stock::stock_data::Data::Status;
use stock::stock_data::Data::Trade;
use stock::stock_data::Data::TradeCancel;
use stock::stock_data::Data::TradeCorrection;
use stock::stock_data::Data::UpdatedBar;

use clickhouse::inserter::Inserter;
use clickhouse::Client;
use clickhouse::Row;
use stock::StockRequest;
use tonic::Request;

pub mod sherpa {
    tonic::include_proto!("sherpa");
}

pub mod stock {
    tonic::include_proto!("stock");
}

pub mod crypto {
    tonic::include_proto!("crypto");
}

pub mod news {
    tonic::include_proto!("news");
}

pub type Result<T> = std::result::Result<T, Error>;
pub type Error = Box<dyn std::error::Error + Send + Sync>;

const PERIOD: u64 = 5;
const MAX_ENTRIES: u64 = 50_000;

struct ClickhouseWriter<T>
where
    T: Row + Serialize,
{
    inserter: Inserter<T>,
    counter: Arc<Mutex<u64>>,
}

impl<T> ClickhouseWriter<T>
where
    T: Row + Serialize,
{
    async fn new(client: Client, table: &str) -> Result<Self> {
        let inserter = client
            .inserter(table)?
            .with_period(Some(Duration::from_secs(PERIOD)))
            .with_max_entries(MAX_ENTRIES);

        Ok(Self {
            inserter,
            counter: Arc::new(Mutex::new(0)),
        })
    }

    async fn write(&mut self, data: &T) -> Result<()> {
        self.inserter.write(data).await?;
        let mut counter = self.counter.lock().unwrap();
        *counter += 1;
        Ok(())
    }

    async fn commit(&mut self) -> Result<Quantities> {
        self.inserter.commit().await.map_err(|e| e.into())
    }
}

impl<T> fmt::Display for ClickhouseWriter<T>
where
    T: Row + Serialize,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let counter = self.counter.lock().unwrap();
        write!(f, "{}", *counter)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // gRPC client
    let mut grpc_client =
        sherpa::stock_streamer_client::StockStreamerClient::connect("http://localhost:10000")
            .await?;

    let req = StockRequest {
        bar: true,
        daily_bar: true,
        luld: true,
        quote: true,
        status: true,
        trade: true,
        trade_cancel: true,
        trade_correction: true,
        updated_bar: true,
    };

    let mut stream = grpc_client.get_stock(Request::new(req)).await?.into_inner();

    // clickhouse

    let clickhouse_client = Client::default()
        .with_url("http://localhost:8123")
        .with_user("default")
        // .with_password("5GqMdPnwLM")
        .with_database("default");

    let sql_folder_path = Path::new("sql");
    for entry in fs::read_dir(sql_folder_path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            println!("running sql script {}", path.to_str().unwrap());
            let sql_statement = fs::read_to_string(&path)?;
            clickhouse_client.query(&sql_statement).execute().await?;
        }
    }

    let mut bar_inserter: ClickhouseWriter<stock::Bar> =
        ClickhouseWriter::new(clickhouse_client.clone(), "bar").await?;
    let bar_counter = bar_inserter.counter.clone();

    let mut daily_bar_inserter: ClickhouseWriter<stock::DailyBar> =
        ClickhouseWriter::new(clickhouse_client.clone(), "daily_bar").await?;
    let daily_bar_counter = daily_bar_inserter.counter.clone();

    let mut trade_inserter: ClickhouseWriter<stock::Trade> =
        ClickhouseWriter::new(clickhouse_client.clone(), "trade").await?;
    let trade_counter = trade_inserter.counter.clone();

    let mut trade_correction_inserter: ClickhouseWriter<stock::TradeCorrection> =
        ClickhouseWriter::new(clickhouse_client.clone(), "trade_correction").await?;
    let trade_correction_counter = trade_correction_inserter.counter.clone();

    let mut trade_cancel_inserter: ClickhouseWriter<stock::TradeCancel> =
        ClickhouseWriter::new(clickhouse_client.clone(), "trade_cancel").await?;
    let trade_cancel_counter = trade_cancel_inserter.counter.clone();

    let mut quote_inserter: ClickhouseWriter<stock::Quote> =
        ClickhouseWriter::new(clickhouse_client.clone(), "quote").await?;
    let quote_counter = quote_inserter.counter.clone();

    let mut updated_bar_inserter: ClickhouseWriter<stock::UpdatedBar> =
        ClickhouseWriter::new(clickhouse_client.clone(), "updated_bar").await?;
    let updated_bar_counter = updated_bar_inserter.counter.clone();

    let mut luld_inserter: ClickhouseWriter<stock::Luld> =
        ClickhouseWriter::new(clickhouse_client.clone(), "luld").await?;
    let luld_counter = luld_inserter.counter.clone();

    let mut status_inserter: ClickhouseWriter<stock::Status> =
        ClickhouseWriter::new(clickhouse_client.clone(), "status").await?;
    let status_counter = status_inserter.counter.clone();

    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            println!(
                "bar {bar_counter} daily_bar {daily_bar_counter} luld {luld_counter} \
                quote {quote_counter} status {status_counter} trade {trade_counter} \
                trade_cancel {trade_cancel_counter} trade_correction {trade_correction_counter} \
                updated_bar {updated_bar_counter}",
                bar_counter = bar_counter.lock().unwrap(),
                daily_bar_counter = daily_bar_counter.lock().unwrap(),
                luld_counter = luld_counter.lock().unwrap(),
                quote_counter = quote_counter.lock().unwrap(),
                status_counter = status_counter.lock().unwrap(),
                trade_counter = trade_counter.lock().unwrap(),
                trade_cancel_counter = trade_cancel_counter.lock().unwrap(),
                trade_correction_counter = trade_correction_counter.lock().unwrap(),
                updated_bar_counter = updated_bar_counter.lock().unwrap(),
            );
        }
    });

    while let Some(message) = stream.message().await? {
        let m = message.data.unwrap();
        match m {
            Bar(bar) => {
                bar_inserter.write(&bar).await?;
                bar_inserter.commit().await?;
            }

            DailyBar(daily_bar) => {
                daily_bar_inserter.write(&daily_bar).await?;
                daily_bar_inserter.commit().await?;
            }
            Trade(trade) => {
                trade_inserter.write(&trade).await?;
                trade_inserter.commit().await?;
            }
            TradeCorrection(trade_correction) => {
                trade_correction_inserter.write(&trade_correction).await?;
                trade_correction_inserter.commit().await?;
            }
            TradeCancel(trade_cancel) => {
                trade_cancel_inserter.write(&trade_cancel).await?;
                trade_cancel_inserter.commit().await?;
            }
            Quote(quote) => {
                quote_inserter.write(&quote).await?;
                quote_inserter.commit().await?;
            }
            UpdatedBar(updated_bar) => {
                updated_bar_inserter.write(&updated_bar).await?;
                updated_bar_inserter.commit().await?;
            }
            Luld(luld) => {
                luld_inserter.write(&luld).await?;
                luld_inserter.commit().await?;
            }
            Status(status) => {
                status_inserter.write(&status).await?;
                status_inserter.commit().await?;
            }
        }
    }
    Ok(())
}
