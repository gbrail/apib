use crate::{collector::Collector, sender::Sender};
use clap::Parser;
use config::Builder;
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
    #[arg(short, help = "Verbose output for every HTTP request")]
    verbose: bool,
    #[arg(help = "The URL to test")]
    url: String,
    #[arg(
        short = 'X',
        help = "HTTP method (default POST when body set, GET otherwise)"
    )]
    method: Option<String>,
    #[arg(short = '1', help = "Send only one request and exit")]
    just_one: bool,
    #[arg(
        short,
        default_value = "1",
        help = "Number of concurrent requests to send"
    )]
    concurrency: u16,
    #[arg(short, default_value = "30", help = "Duration of test run")]
    duration: u16,
    #[arg(short = 't', help = "Data to send in HTTP body")]
    body_text: Option<String>,
    #[arg(short = 'T', help = "File to read HTTP body from")]
    body_file: Option<String>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let mut builder = Builder::new().set_url(&args.url).set_verbose(args.verbose);
    if let Some(m) = &args.method {
        builder = builder.set_method(m);
    }
    if let Some(bt) = &args.body_text {
        builder = builder.set_body_text(bt);
    }
    if let Some(bf) = &args.body_file {
        builder = builder.set_body_file(bf);
    }
    let config = match builder.build().await {
        Ok(cfg) => Arc::new(cfg),
        Err(e) => {
            println!("Invalid configuration: {}", e);
            return;
        }
    };

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
