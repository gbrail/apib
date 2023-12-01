use crate::collector::Collector;
use crate::sender::Sender;
use clap::Parser;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::mpsc;

mod collector;
mod error;
mod sender;

const TICK_DURATION: Duration = Duration::from_secs(5);

#[derive(Parser, Debug)]
struct Args {
    #[arg(short)]
    verbose: bool,
    #[arg()]
    url: String,
    #[arg(short, default_value = "1")]
    concurrency: u16,
    #[arg(short, default_value = "30")]
    duration: u16,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let collector = Arc::new(Collector::new());
    let mut sender = Sender::new(&args.url).expect("Error initializing reqwest");
    sender.set_verbose(args.verbose);

    let sc = Arc::new(sender);
    let (send_done, mut recv_done) = mpsc::unbounded_channel();
    let start_time = SystemTime::now();
    let test_duration = Duration::from_secs(args.duration as u64);

    for _ in 0..args.concurrency {
        let coll = Arc::clone(&collector);
        let done = send_done.clone();
        let s = Arc::clone(&sc);
        tokio::spawn(async move {
            s.do_loop(coll.as_ref()).await;
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
