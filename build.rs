use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    tonic_build::configure()
        .file_descriptor_set_path(out_dir.join("sherpa_descriptor.bin"))
        .message_attribute(
            "stock.DailyBar",
            "#[derive(clickhouse::Row, serde::Serialize)]",
        )
        .message_attribute(
            "stock.Quote",
            "#[derive(clickhouse::Row, serde::Serialize)]",
        )
        .message_attribute(
            "stock.Status",
            "#[derive(clickhouse::Row, serde::Serialize)]",
        )
        .message_attribute(
            "stock.Trade",
            "#[derive(clickhouse::Row, serde::Serialize)]",
        )
        .message_attribute(
            "stock.TradeCancel",
            "#[derive(clickhouse::Row, serde::Serialize)]",
        )
        .message_attribute(
            "stock.TradeCorrection",
            "#[derive(clickhouse::Row, serde::Serialize)]",
        )
        .message_attribute(
            "stock.UpdatedBar",
            "#[derive(clickhouse::Row, serde::Serialize)]",
        )
        .message_attribute("stock.Bar", "#[derive(clickhouse::Row, serde::Serialize)]")
        .message_attribute("stock.Luld", "#[derive(clickhouse::Row, serde::Serialize)]")
        .compile(
            &[
                "proto/sherpa.proto",
                "proto/stock.proto",
                "proto/news.proto",
                "proto/crypto.proto",
            ],
            &["proto"],
        )?;

    Ok(())
}
