use apib::{Builder, Collector};
use clap::Parser;
use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::sync::mpsc;

/*
 * TODO:
 * * Params for closing connections on client side after a timeout or # requests
 * * Think time
 * * CSV output and title
 * * HTTP/2 streaming options
 * * Track number of bytes
 */

#[derive(Parser, Debug)]
struct Args {
    #[arg(help = "The URL to test")]
    url: String,
    #[arg(short, long, help = "Verbose output for every HTTP request")]
    verbose: bool,
    #[arg(
        short = 'X',
        long,
        help = "HTTP method (default POST when body set, GET otherwise)"
    )]
    method: Option<String>,
    #[arg(short = '1', long = "one", help = "Send only one request and exit")]
    just_one: bool,
    #[arg(
        short,
        long,
        default_value = "1",
        help = "Number of concurrent requests to send"
    )]
    concurrency: u16,
    #[arg(
        short,
        long,
        default_value = "30",
        help = "Duration of test run in seconds"
    )]
    duration: u16,
    #[arg(
        short,
        long,
        default_value = "0",
        help = "Warm-up time before test run, in seconds"
    )]
    warmup: u16,
    #[arg(
        long,
        default_value = "5",
        help = "How often to print intermediate results, in seconds"
    )]
    print_interval: u16,
    #[arg(short = 't', long, help = "Data to send in HTTP body")]
    body_text: Option<String>,
    #[arg(short = 'T', long, help = "File to read HTTP body from")]
    body_file: Option<String>,
    #[arg(
        short = 'H',
        long = "header",
        help = "Header name to add, in name:value format"
    )]
    headers: Vec<String>,
    #[arg(
        short = 'k',
        long = "insecure",
        help = "Skip verification of TLS certificates"
    )]
    skip_tls_verify: bool,
    #[arg(short = '2', long = "http2", help = "Force HTTP/2 connection")]
    http2: bool,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let mut builder = Builder::new()
        .set_url(&args.url)
        .set_verbose(args.verbose)
        .set_tls_no_verify(args.skip_tls_verify)
        .set_http2(args.http2);
    if let Some(m) = &args.method {
        builder = builder.set_method(m);
    }
    if let Some(bt) = &args.body_text {
        builder = builder.set_body_text(bt);
    }
    if let Some(bf) = &args.body_file {
        builder = builder.set_body_file(bf);
    }
    for hdr_val in &args.headers {
        let hdr_split: Vec<&str> = hdr_val.splitn(2, ':').collect();
        builder = builder.add_header(hdr_split[0], hdr_split[1]);
    }
    let config = match builder.build().await {
        Ok(cfg) => Arc::new(cfg),
        Err(e) => {
            println!("Invalid configuration: {}", e);
            return;
        }
    };

    let collector = Arc::new(Collector::new());

    // If the "-1" argument was used, just send one and exit.
    if args.just_one {
        let mut sender = apib::new_sender(config, collector, args.http2);
        if let Err(e) = sender.send().await {
            println!("Error on send: {}", e);
        }
        return;
    }

    let (send_done, mut recv_done) = mpsc::unbounded_channel();
    let mut start_time = SystemTime::now();
    let test_duration = Duration::from_secs(args.duration as u64);
    let warmup_duration = Duration::from_secs(args.warmup as u64);
    if !warmup_duration.is_zero() {
        collector.set_warming_up(true);
    }
    let total_duration = test_duration + warmup_duration;
    let print_interval = Duration::from_secs(args.print_interval as u64);

    // Spawn an async task based on the concurrency level. Each task will run
    // until the collector tells it to stop, and then send a message on
    // the channel.
    for _ in 0..args.concurrency {
        let local_collector = Arc::clone(&collector);
        let local_config = Arc::clone(&config);
        let done = send_done.clone();
        tokio::spawn(async move {
            let mut sender = apib::new_sender(local_config, local_collector, args.http2);
            sender.do_loop().await;
            done.send(true).unwrap();
        });
    }

    // Spawn the task that will periodically print the progress of the test
    // run. This is currently every five seconds.
    let tick_coll = Arc::clone(&collector);
    tokio::spawn(async move {
        while !tick_coll.stopped() {
            let tick_start = SystemTime::now();
            tokio::time::sleep(print_interval).await;
            tick_coll.write_tick(start_time, tick_start, total_duration);
        }
    });

    // Do warmup time if we have to.
    if !warmup_duration.is_zero() {
        tokio::time::sleep(warmup_duration).await;
        start_time = SystemTime::now();
        collector.set_warming_up(false);
    }

    // Wait for the planned duration of the test run.
    tokio::time::sleep(test_duration).await;

    // Stop, and wait for each test task to send a message indicating that
    // it is done.
    collector.stop();
    let stop_time = SystemTime::now();
    for _ in 0..args.concurrency {
        recv_done.recv().await.unwrap();
    }

    // Statistics from each test task are done so write them all out here.
    let results = collector.get_results(start_time, stop_time);
    results.write();
}
