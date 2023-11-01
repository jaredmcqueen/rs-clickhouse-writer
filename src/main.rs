use clickhouse::inserter::Quantities;
use serde::Serialize;
use std::env;
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
use tokio::time::interval;

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
    async fn new(client: Client, table: &str, period: u64) -> Result<Self> {
        let inserter = client
            .inserter(table)?
            .with_period(Some(Duration::from_secs(period)));

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
        // self.inserter.commit().await.map_err(|e| e.into())
        let stats = self.inserter.commit().await?;
        let name = std::any::type_name::<Self>();
        if stats.entries > 0 {
            println!("{} sent {}", name, stats.entries);
        }
        Ok(stats)
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
    let sherpa_endpoint = env::var("SHERPA_ENDPOINT").expect("SHERPA_ENDPOINT must be set");
    let clickhouse_endpoint =
        env::var("CLICKHOUSE_ENDPOINT").expect("CLICKHOUSE_ENDPOINT must be set");
    let clickhouse_username =
        env::var("CLICKHOUSE_USERNAME").expect("CLICKHOUSE_USERNAME must be set");
    let clickhouse_password =
        env::var("CLICKHOUSE_PASSWORD").expect("CLICKHOUSE_PASSWORD must be set");
    let clickhouse_database =
        env::var("CLICKHOUSE_DATABASE").expect("CLICKHOUSE_DATABASE must be set");
    let period = env::var("PERIOD")
        .expect("PERIOD must be set")
        .parse::<u64>()
        .expect("could not parse PERIOD into u64");

    // gRPC client
    let mut grpc_client =
        sherpa::stock_streamer_client::StockStreamerClient::connect(sherpa_endpoint).await?;

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
        .with_url(clickhouse_endpoint)
        .with_user(clickhouse_username)
        .with_password(clickhouse_password)
        .with_database(clickhouse_database);

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
        ClickhouseWriter::new(clickhouse_client.clone(), "bar", period).await?;

    let mut daily_bar_inserter: ClickhouseWriter<stock::DailyBar> =
        ClickhouseWriter::new(clickhouse_client.clone(), "daily_bar", period).await?;

    let mut trade_inserter: ClickhouseWriter<stock::Trade> =
        ClickhouseWriter::new(clickhouse_client.clone(), "trade", period).await?;

    let mut trade_correction_inserter: ClickhouseWriter<stock::TradeCorrection> =
        ClickhouseWriter::new(clickhouse_client.clone(), "trade_correction", period).await?;

    let mut trade_cancel_inserter: ClickhouseWriter<stock::TradeCancel> =
        ClickhouseWriter::new(clickhouse_client.clone(), "trade_cancel", period).await?;

    let mut quote_inserter: ClickhouseWriter<stock::Quote> =
        ClickhouseWriter::new(clickhouse_client.clone(), "quote", period).await?;

    let mut updated_bar_inserter: ClickhouseWriter<stock::UpdatedBar> =
        ClickhouseWriter::new(clickhouse_client.clone(), "updated_bar", period).await?;

    let mut luld_inserter: ClickhouseWriter<stock::Luld> =
        ClickhouseWriter::new(clickhouse_client.clone(), "luld", period).await?;

    let mut status_inserter: ClickhouseWriter<stock::Status> =
        ClickhouseWriter::new(clickhouse_client.clone(), "status", period).await?;

    let mut commit_interval = interval(Duration::from_secs(1));

    loop {
        tokio::select! {
            message_result = stream.message() => {
                if let Some(message) = message_result? {
                    let m = message.data.unwrap();
                    match m {
                        Bar(bar) => {
                            bar_inserter.write(&bar).await?;
                        }
                        DailyBar(daily_bar) => {
                            daily_bar_inserter.write(&daily_bar).await?;
                        }
                        Trade(trade) => {
                            trade_inserter.write(&trade).await?;
                        }
                        TradeCorrection(trade_correction) => {
                            trade_correction_inserter.write(&trade_correction).await?;
                        }
                        TradeCancel(trade_cancel) => {
                            trade_cancel_inserter.write(&trade_cancel).await?;
                        }
                        Quote(quote) => {
                            quote_inserter.write(&quote).await?;
                        }
                        UpdatedBar(updated_bar) => {
                            updated_bar_inserter.write(&updated_bar).await?;
                        }
                        Luld(luld) => {
                            luld_inserter.write(&luld).await?;
                        }
                        Status(status) => {
                            status_inserter.write(&status).await?;
                        }
                    }
                }
            }
            _ = commit_interval.tick() => {
                bar_inserter.commit().await?;
                daily_bar_inserter.commit().await?;
                trade_inserter.commit().await?;
                trade_correction_inserter.commit().await?;
                trade_cancel_inserter.commit().await?;
                quote_inserter.commit().await?;
                updated_bar_inserter.commit().await?;
                luld_inserter.commit().await?;
                status_inserter.commit().await?;

            println!( "bar {bar_inserter} daily_bar {daily_bar_inserter} luld {luld_inserter} \
                quote {quote_inserter} status {status_inserter} trade {trade_inserter} \
                trade_cancel {trade_cancel_inserter} trade_correction {trade_correction_inserter} \
                updated_bar {updated_bar_inserter}");
            }
        }
    }
}
