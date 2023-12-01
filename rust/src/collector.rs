use crate::error::Error;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

#[derive(Default)]
struct Stats {
    attempts: u32,
    successes: u32,
    failures: u32,
    bytes_sent: u64,
    bytes_received: u64,
    total_latency: Duration,
}

#[derive(Default)]
pub struct Collector {
    stopped: AtomicBool,
    interval_successes: AtomicU32,
    interval_failures: AtomicU32,
    last_error: Mutex<Option<Error>>,
    stats: Mutex<Stats>,
}

impl Collector {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn stop(&self) {
        self.stopped.store(true, Ordering::Relaxed);
    }

    pub fn stopped(&self) -> bool {
        self.stopped.load(Ordering::Relaxed)
    }

    pub fn success(&self) -> bool {
        self.interval_successes.fetch_add(1, Ordering::Relaxed);
        self.stopped()
    }

    pub fn failure(&self, err: Error) -> bool {
        self.interval_failures.fetch_add(1, Ordering::Relaxed);
        {
            let mut last_err = self.last_error.lock().unwrap();
            *last_err = Some(err);
        }
        self.stopped()
    }

    pub fn collect(&self, local: LocalCollector) {
        let mut stats = self.stats.lock().unwrap();
        stats.attempts += local.stats.attempts;
        stats.successes += local.stats.successes;
        stats.failures += local.stats.failures;
        stats.bytes_sent += local.stats.bytes_sent;
        stats.bytes_received += local.stats.bytes_received;
        stats.total_latency += local.stats.total_latency;
    }

    pub fn write(&self, start: SystemTime, end: SystemTime) {
        let duration = end
            .duration_since(start)
            .expect("Error calculating duration");
        let stats = self.stats.lock().unwrap();

        println!("Duration:            {}", duration.as_secs_f64());
        println!("Attempted requests:  {}", stats.attempts);
        println!("Successful requests: {}", stats.successes);
        println!("Errors:              {}", stats.failures);
        println!();
        println!(
            "Throughput:          {} requests/second",
            get_throughput(stats.successes, &duration)
        );
    }

    pub fn write_tick(&self, start: SystemTime, tick_start: SystemTime, test_duration: Duration) {
        let now = SystemTime::now();
        let so_far = now
            .duration_since(start)
            .expect("Error calculating duration");
        let interval_duration = now
            .duration_since(tick_start)
            .expect("Error calculating duration");

        let interval_successes = self.interval_successes.swap(0, Ordering::Relaxed);
        let interval_failures = self.interval_failures.swap(0, Ordering::Relaxed);
        let throughput = get_throughput(interval_successes, &interval_duration);
        let last_error = {
            let mut last_err_ref = self.last_error.lock().unwrap();
            let last_error = last_err_ref.clone();
            *last_err_ref = None;
            last_error
        };

        if let Some(err) = last_error {
            println!(
                "({} / {}) {} ({} errors)",
                so_far.as_secs(),
                test_duration.as_secs(),
                throughput,
                interval_failures
            );
            println!("  {}", err);
        } else {
            println!(
                "({} / {}) {}",
                so_far.as_secs(),
                test_duration.as_secs(),
                throughput
            );
        }
    }
}

fn get_throughput(successes: u32, duration: &Duration) -> f64 {
    successes as f64 / duration.as_secs_f64()
}

#[derive(Default)]
pub struct LocalCollector {
    stats: Stats,
}

impl LocalCollector {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn success(&mut self, start: SystemTime, sent: u64, received: u64) {
        let latency = SystemTime::now()
            .duration_since(start)
            .expect("Error getting current time");
        self.stats.attempts += 1;
        self.stats.successes += 1;
        self.stats.bytes_sent += sent;
        self.stats.bytes_received += received;
        self.stats.total_latency += latency;
    }

    pub fn failure(&mut self) {
        self.stats.attempts += 1;
        self.stats.failures += 1;
    }
}
