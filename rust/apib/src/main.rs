use crate::{collector::Collector, config::Config, sender::Sender};
use clap::Parser;
use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::sync::mpsc;

mod collector;
mod config;
mod error;
mod null_verifier;
mod sender;

const TICK_DURATION: Duration = Duration::from_secs(5);

#[derive(Parser, Debug)]
struct Args {
    #[arg(short)]
    verbose: bool,
    #[arg()]
    url: String,
    #[arg(short = '1')]
    just_one: bool,
    #[arg(short, default_value = "1")]
    concurrency: u16,
    #[arg(short, default_value = "30")]
    duration: u16,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let mut raw_config = match Config::new(&args.url) {
        Ok(cfg) => cfg,
        Err(e) => {
            println!("Invalid configuration: {}", e);
            return;
        }
    };
    raw_config.set_verbose(args.verbose);

    let config = Arc::new(raw_config);
    let collector = Arc::new(Collector::new());
    let mut sender = Sender::new(Arc::clone(&config));

    if args.just_one {
        if let Err(e) = sender.send().await {
            println!("Error on send: {}", e);
        }
        return;
    }

    let (send_done, mut recv_done) = mpsc::unbounded_channel();
    let start_time = SystemTime::now();
    let test_duration = Duration::from_secs(args.duration as u64);

    for _ in 0..args.concurrency {
        let local_collector = Arc::clone(&collector);
        let local_config = Arc::clone(&config);
        let done = send_done.clone();
        tokio::spawn(async move {
            let mut sender = Sender::new(local_config);
            sender.do_loop(local_collector.as_ref()).await;
            done.send(true).unwrap();
        });
    }

    let tick_coll = Arc::clone(&collector);
    tokio::spawn(async move {
        while !tick_coll.stopped() {
            let tick_start = SystemTime::now();
            tokio::time::sleep(TICK_DURATION).await;
            tick_coll.write_tick(start_time, tick_start, test_duration);
        }
    });

    tokio::time::sleep(test_duration).await;
    collector.stop();
    for _ in 0..args.concurrency {
        recv_done.recv().await.unwrap();
    }
    collector.write(start_time, SystemTime::now());
}
