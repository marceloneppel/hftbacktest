use std::time::Instant;

use algo::gridtrading;
use clap::Parser;
use hftbacktest::{
    backtest::{
        assettype::LinearAsset,
        models::{IntpOrderLatency, PowerProbQueueFunc3, ProbQueueModel, QueuePos},
        recorder::BacktestRecorder,
        reader::read_npz,
        AssetBuilder,
        DataSource,
        ExchangeKind,
        MultiAssetMultiExchangeBacktest,
    },
    prelude::{HashMapMarketDepth, Interface, ApplySnapshot},
};

mod algo;

fn prepare_backtest() -> MultiAssetMultiExchangeBacktest<QueuePos, HashMapMarketDepth> {
    let latency_data = (20240501..20240532)
        .map(|date| DataSource::File(format!("latency_{date}.npz")))
        .collect();

    let latency_model = IntpOrderLatency::new(latency_data).unwrap();
    let asset_type = LinearAsset::new(1.0);
    let queue_model = ProbQueueModel::new(PowerProbQueueFunc3::new(3.0));

    let data = (20240501..20240532)
        .map(|date| DataSource::File(format!("1000SHIBUSDT_{date}.npz")))
        .collect();

    let hbt = MultiAssetMultiExchangeBacktest::builder()
        .add(
            AssetBuilder::new()
                .data(data)
                .latency_model(latency_model)
                .asset_type(asset_type)
                .maker_fee(-0.00005)
                .taker_fee(0.0007)
                .queue_model(queue_model)
                .depth(|| {
                    let mut depth = HashMapMarketDepth::new(0.000001, 1.0);
                    depth.apply_snapshot(&read_npz("1000SHIBUSDT_20240501_SOD.npz").unwrap());
                    depth
                })
                .exchange(ExchangeKind::NoPartialFillExchange)
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();
    hbt
}

fn main() {
    tracing_subscriber::fmt::init();

    let relative_half_spread = 0.0005;
    let relative_grid_interval = 0.0005;
    let grid_num = 10;
    let skew = relative_half_spread / grid_num as f64;
    let order_qty = 1.0;

    let mut start = Instant::now();
    let mut hbt = prepare_backtest();
    let mut recorder = BacktestRecorder::new(&hbt);
    gridtrading(
        &mut hbt,
        &mut recorder,
        relative_half_spread,
        relative_grid_interval,
        grid_num,
        skew,
        order_qty,
    )
    .unwrap();
    hbt.close().unwrap();
    print!("{} seconds", start.elapsed().as_secs());
    recorder.to_csv("gridtrading", ".").unwrap();
}
