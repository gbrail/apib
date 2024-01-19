use crate::error::Error;
use std::cmp::min;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

const MEGABIT: f64 = 1024.0 * 1024.0 * 10.0;

#[derive(Default)]
struct Stats {
    attempts: u32,
    successes: u32,
    failures: u32,
    connections_opened: u32,
    bytes_sent: u64,
    bytes_received: u64,
    total_latency: Duration,
    latencies: Vec<Duration>,
}

#[derive(Default)]
pub struct Collector {
    stopped: AtomicBool,
    warming_up: AtomicBool,
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

    pub fn set_warming_up(&self, warming: bool) {
        self.warming_up.store(warming, Ordering::Relaxed);
    }

    pub fn stopped(&self) -> bool {
        self.stopped.load(Ordering::Relaxed)
    }

    pub fn success(&self, local: &mut LocalCollector, start: SystemTime, end: SystemTime) -> bool {
        self.interval_successes.fetch_add(1, Ordering::Relaxed);
        if !self.warming_up.load(Ordering::Relaxed) {
            local.success(start, end);
        }
        self.stopped()
    }

    pub fn failure(&self, local: &mut LocalCollector, err: Error) -> bool {
        self.interval_failures.fetch_add(1, Ordering::Relaxed);
        {
            let mut last_err = self.last_error.lock().unwrap();
            *last_err = Some(err);
        }
        if !self.warming_up.load(Ordering::Relaxed) {
            local.failure();
        }
        self.stopped()
    }

    pub fn connection_opened(local: &mut LocalCollector) {
        local.connection_opened();
    }

    // Merge detailed local stats with the total set of stats across all tasks.
    pub fn collect(&self, mut local: LocalCollector) {
        let mut stats = self.stats.lock().unwrap();
        stats.attempts += local.stats.attempts;
        stats.successes += local.stats.successes;
        stats.failures += local.stats.failures;
        stats.connections_opened += local.stats.connections_opened;
        stats.bytes_sent += local.stats.bytes_sent;
        stats.bytes_received += local.stats.bytes_received;
        stats.total_latency += local.stats.total_latency;
        stats.latencies.append(&mut local.stats.latencies);
    }

    pub fn collect_connection(&self, bytes_sent: u64, bytes_received: u64) {
        let mut stats = self.stats.lock().unwrap();
        stats.bytes_sent += bytes_sent;
        stats.bytes_received += bytes_received;
    }

    pub fn get_results(&self, start: SystemTime, end: SystemTime) -> Results {
        let duration = end
            .duration_since(start)
            .expect("Error calculating duration");

        let mut stats = self.stats.lock().unwrap();
        stats.latencies.sort();

        let mut latency_pct = vec![];
        for i in 0..101 {
            latency_pct.push(get_latency(stats.latencies.as_ref(), i));
        }

        Results {
            duration_secs: duration.as_secs_f64(),
            attempts: stats.attempts,
            successes: stats.successes,
            failures: stats.failures,
            connections_opened: stats.connections_opened,
            throughput: get_throughput(stats.successes, &duration),
            latency_avg: get_avg_latency(stats.successes, &stats.total_latency),
            latency_pct,
            bytes_sent: stats.bytes_sent,
            send_rate: get_rate(stats.bytes_sent, &duration),
            bytes_received: stats.bytes_received,
            receive_rate: get_rate(stats.bytes_received, &duration),
        }
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

        let warmup = if self.warming_up.load(Ordering::Relaxed) {
            " (warming up)"
        } else {
            ""
        };
        if let Some(err) = last_error {
            println!(
                "({} / {}) {:.3} {} ({} errors)\n  {}",
                so_far.as_secs(),
                test_duration.as_secs(),
                throughput,
                warmup,
                interval_failures,
                err
            )
        } else {
            println!(
                "({} / {}) {:.3} {}",
                so_far.as_secs(),
                test_duration.as_secs(),
                throughput,
                warmup
            )
        };
    }
}

fn get_throughput(successes: u32, duration: &Duration) -> f64 {
    if duration.is_zero() {
        return 0.0;
    }
    successes as f64 / duration.as_secs_f64()
}

fn get_rate(bytes: u64, duration: &Duration) -> f64 {
    if duration.is_zero() {
        return 0.0;
    }
    bytes as f64 / MEGABIT / duration.as_secs_f64()
}

fn get_avg_latency(successes: u32, duration: &Duration) -> f64 {
    if successes == 0 {
        return 0.0;
    }
    duration.as_secs_f64() * 1000.0 / successes as f64
}

fn get_latency(latencies: &[Duration], percent: usize) -> f64 {
    if latencies.is_empty() {
        return 0.0;
    }
    let ix = min(latencies.len() * percent / 100, latencies.len() - 1);
    latencies[ix].as_secs_f64() * 1000.0
}

#[derive(Default)]
pub struct LocalCollector {
    stats: Stats,
}

impl LocalCollector {
    pub fn new() -> Self {
        Default::default()
    }

    fn success(&mut self, start: SystemTime, end: SystemTime) {
        let latency = end
            .duration_since(start)
            .expect("Error getting current time");
        self.stats.attempts += 1;
        self.stats.successes += 1;
        self.stats.total_latency += latency;
        self.stats.latencies.push(latency);
    }

    fn failure(&mut self) {
        self.stats.attempts += 1;
        self.stats.failures += 1;
    }

    fn connection_opened(&mut self) {
        self.stats.connections_opened += 1;
    }
}

#[derive(Default)]
pub struct Results {
    pub duration_secs: f64,
    pub attempts: u32,
    pub successes: u32,
    pub failures: u32,
    pub connections_opened: u32,
    pub throughput: f64,
    pub latency_avg: f64,
    pub latency_pct: Vec<f64>,
    pub bytes_sent: u64,
    pub send_rate: f64,
    pub bytes_received: u64,
    pub receive_rate: f64,
}

impl Results {
    pub fn write(&self) {
        println!("Duration:            {:.3} seconds", self.duration_secs);
        println!("Attempted requests:  {}", self.attempts);
        println!("Successful requests: {}", self.successes);
        println!("Errors:              {}", self.failures);
        println!("Connections opened:  {}", self.connections_opened);
        println!();
        println!(
            "Throughput:          {:.3} requests/second",
            self.throughput
        );
        println!("Average latency:     {:.3} milliseconds", self.latency_avg);
        println!(
            "Minimum latency:     {:.3} milliseconds",
            self.latency_pct[0]
        );
        println!(
            "Maximum latency:     {:.3} milliseconds",
            self.latency_pct[100]
        );
        println!(
            "50% latency:         {:.3} milliseconds",
            self.latency_pct[50]
        );
        println!(
            "90% latency:         {:.3} milliseconds",
            self.latency_pct[90]
        );
        println!(
            "98% latency:         {:.3} milliseconds",
            self.latency_pct[98]
        );
        println!(
            "99% latency:         {:.3} milliseconds",
            self.latency_pct[99]
        );
        println!("Bytes Sent:          {} bytes", self.bytes_sent);
        println!(
            "Send rate:           {:.3} megabits / second",
            self.send_rate
        );
        println!("Bytes Received:      {} bytes", self.bytes_received);
        println!(
            "Receive rate:        {:.3} megabits / second",
            self.receive_rate
        );
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_zero() {
        let local = LocalCollector::new();
        let global = Collector::new();
        let start = SystemTime::now();
        global.collect(local);
        let results = global.get_results(start, SystemTime::now());
        assert_eq!(results.successes, 0);
        assert_eq!(results.attempts, 0);
        assert_eq!(results.failures, 0);
    }

    #[test]
    fn test_one_success() {
        let mut local = LocalCollector::new();
        let global = Collector::new();
        let start = SystemTime::now();
        let elapsed = Duration::from_millis(111);
        let end = start.checked_add(elapsed).expect("Error adding time");
        assert_eq!(global.success(&mut local, start, end), false);
        global.collect(local);
        let results = global.get_results(start, SystemTime::now());
        assert_eq!(results.successes, 1);
        assert_eq!(results.attempts, 1);
        assert_eq!(results.failures, 0);
        let elapsed_millis = elapsed.as_secs_f64() * 1000.0;
        assert_eq!(results.latency_avg, elapsed_millis);
        assert_eq!(results.latency_pct[0], elapsed_millis);
        assert_eq!(results.latency_pct[100], elapsed_millis);
    }

    #[test]
    fn test_percentages() {
        let mut local = LocalCollector::new();
        let global = Collector::new();
        let start = SystemTime::now();
        let elapsed = Duration::from_millis(111);
        let end = start.checked_add(elapsed).expect("Error adding time");
        assert_eq!(global.success(&mut local, start, end), false);
        let elapsed2 = Duration::from_millis(222);
        let end2 = start.checked_add(elapsed2).expect("Error adding time");
        assert_eq!(global.success(&mut local, start, end2), false);
        let elapsed3 = Duration::from_millis(33);
        let end3 = start.checked_add(elapsed3).expect("Error adding time");
        assert_eq!(global.success(&mut local, start, end3), false);
        global.collect(local);
        let results = global.get_results(start, SystemTime::now());
        assert_eq!(results.successes, 3);
        assert_eq!(results.attempts, 3);
        assert_eq!(results.failures, 0);
        assert_eq!(results.latency_avg, 122.0);
        assert_eq!(results.latency_pct[0], 33.0);
        assert_eq!(results.latency_pct[100], 222.0);
    }

    #[test]
    fn test_one_failure() {
        let mut local = LocalCollector::new();
        let global = Collector::new();
        let start = SystemTime::now();
        global.failure(&mut local, Error::IO("Help!".to_string()));
        global.collect(local);
        let results = global.get_results(start, SystemTime::now());
        assert_eq!(results.successes, 0);
        assert_eq!(results.attempts, 1);
        assert_eq!(results.failures, 1);
    }
}
