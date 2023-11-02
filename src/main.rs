use anyhow::anyhow;
use chrono::{prelude::*, Days};
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::str::FromStr;
use tracing::Level;
use tracing::{error, info};
use tracing_appender::non_blocking::NonBlocking;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::SubscriberBuilder;

use rusqlite::Connection;

extern crate tokio;

pub type PerformanceMetrics = Vec<PerformanceData>;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceData {
    pub battery_solar: Option<f64>,
    pub consumption: Option<Value>,
    pub export_reading: Option<Value>,
    pub estimated_solar: Option<Value>,
    pub import_reading: Option<Value>,
    pub pv_solar: Option<f64>,
    pub self_consumption: Option<Value>,
    pub solar: Option<f64>,
    pub system_production: Option<f64>,
    pub timestamp: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolarData {
    pub pv_solar: f64,
    pub solar: f64,
    pub timestamp: String,
}

impl From<PerformanceData> for SolarData {
    fn from(data: PerformanceData) -> Self {
        SolarData {
            pv_solar: data.pv_solar.unwrap(),
            solar: data.solar.unwrap(),
            timestamp: data.timestamp,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub prospect_id: String,
    pub jwt_token: String,
    pub log_level: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config: Config = match fs::read_to_string("data.hcl") {
        Ok(data) => match hcl::from_str(data.as_str()) {
            Ok(config) => config,
            Err(e) => {
                println!("Error: {}", e.to_string());
                return Err(anyhow!(e.to_string()));
            }
        },
        Err(e) => {
            println!("Error: {}", e.to_string());
            return Err(anyhow!(e.to_string()));
        }
    };

    let log_level = match config.log_level {
        Some(level) => level,
        None => "INFO".to_string(),
    };

    let level: tracing::Level = Level::from_str(log_level.as_str()).unwrap();
    let subscriber: SubscriberBuilder = tracing_subscriber::fmt();
    let non_blocking: NonBlocking;
    let _guard: WorkerGuard;
    (non_blocking, _guard) = tracing_appender::non_blocking(std::io::stdout());

    subscriber
        .with_writer(non_blocking)
        .with_max_level(level)
        .with_level(true)
        .with_line_number(level == tracing::Level::TRACE)
        .with_file(level == tracing::Level::TRACE)
        .compact()
        .init();

    let client = match reqwest::Client::builder()
        .brotli(true)
        .gzip(true)
        .https_only(true)
        .connection_verbose(true)
        .use_rustls_tls()
        .build()
    {
        Ok(client) => client,
        Err(e) => {
            return Err(anyhow!(e.to_string()));
        }
    };

    let mut headers = HeaderMap::new();
    headers.insert(
        "Authorization",
        format!("{}", config.jwt_token).parse().unwrap(),
    );
    headers.insert("Pragma", "no-cache".parse().unwrap());
    headers.insert("Cache-Control", "no-cache".parse().unwrap());

    let now: DateTime<Local> = Local::now().checked_add_days(Days::new(1)).unwrap();
    let past: DateTime<Local> = now.checked_sub_days(Days::new(7)).unwrap();

    let end_ts = now.to_rfc3339();
    let start_ts: String = past.to_rfc3339();

    let results = match client
        .get(format!(
            "https://gateway.sunrun.com/performance-api/v1/site-production-minute/{}",
            config.prospect_id,
        ))
        .query(&[("startDate", start_ts), ("endDate", end_ts)])
        .headers(headers)
        .send()
        .await
    {
        Ok(response) => match response.json::<PerformanceMetrics>().await {
            Ok(data) => Some(data),
            Err(_) => None,
        },
        Err(e) => {
            error!("{}", e.to_string());
            None
        }
    };

    match results {
        Some(data) => {
            info!("Fetched {} Records", data.len());
            let connection = Connection::open(Path::new("sunrun.sqlite3"))?;
            connection.execute("CREATE TABLE IF NOT EXISTS solar (pv_solar NUMERIC, solar NUMBERIC, timestamp DATETIME UNIQUE)", ())?;
            for row in data {
                let s: SolarData = row.into();
                match connection.execute(
                    "INSERT INTO solar (pv_solar, solar, timestamp) VALUES (?1, ?2, ?3);",
                    (&s.pv_solar, &s.solar, &s.timestamp),
                ) {
                    Ok(_) => {}
                    Err(_) => {}
                };
            }

            connection.close().unwrap();
        }
        None => {
            error!("No data received from Sunrun API. Check if API key has expired?")
        }
    }
    return Ok(());
}
