use apib::{new_sender, Builder, Collector};
use httptarget::Target;
use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::sync::mpsc;

const TEST_DURATION: Duration = Duration::from_millis(500);

#[tokio::test]
async fn test_get() {
    let mut target = make_target(false).await;
    let address = target.address();
    let config = Arc::new(
        Builder::new()
            .set_url(&format!("http://127.0.0.1:{}/hello", address.port()))
            .build()
            .await
            .expect("Error building config"),
    );
    let collector = Arc::new(Collector::new());
    let mut sender = new_sender(config, collector, false);
    sender.send().await.expect("Expected no error");
    target.stop();
}

#[tokio::test]
async fn test_get_https() {
    let mut target = make_target(true).await;
    let address = target.address();
    let config = Arc::new(
        Builder::new()
            .set_url(&format!("https://127.0.0.1:{}/hello", address.port()))
            .set_tls_no_verify(true)
            .build()
            .await
            .expect("Error building config"),
    );
    let collector = Arc::new(Collector::new());
    let mut sender = new_sender(config, collector, false);
    sender.send().await.expect("Expected no error");
    target.stop();
}

#[tokio::test]
async fn test_get_http2_forced() {
    let mut target = make_target(false).await;
    let address = target.address();
    let config = Arc::new(
        Builder::new()
            .set_url(&format!("http://127.0.0.1:{}/hello", address.port()))
            .set_http2(true)
            .build()
            .await
            .expect("Error building config"),
    );
    let collector = Arc::new(Collector::new());
    let mut sender = new_sender(config, collector, true);
    sender.send().await.expect("Expected no error");
    target.stop();
}

#[tokio::test]
async fn test_post() {
    let mut target = make_target(false).await;
    let address = target.address();
    let config = Arc::new(
        Builder::new()
            .set_url(&format!("http://127.0.0.1:{}/echo", address.port()))
            .set_method("POST")
            .set_body_text("Hello, Server!")
            .build()
            .await
            .expect("Error building config"),
    );
    let collector = Arc::new(Collector::new());
    let mut sender = new_sender(config, collector, false);
    sender.send().await.expect("Expected no error");
    target.stop();
}

#[tokio::test]
async fn test_not_found() {
    let mut target = make_target(false).await;
    let address = target.address();
    let config = Arc::new(
        Builder::new()
            .set_url(&format!("http://127.0.0.1:{}/NOTFOUND", address.port()))
            .build()
            .await
            .expect("Error building config"),
    );
    let collector = Arc::new(Collector::new());
    let mut sender = new_sender(config, collector, false);
    assert!(sender.send().await.is_err());
    target.stop();
}

#[tokio::test]
async fn test_loops() {
    let mut target = make_target(false).await;
    let address = target.address();
    let config = Arc::new(
        Builder::new()
            .set_url(&format!("http://127.0.0.1:{}/hello", address.port()))
            .build()
            .await
            .expect("Error building config"),
    );
    let collector = Arc::new(Collector::new());
    let (send, mut recv) = mpsc::unbounded_channel();
    let start = SystemTime::now();

    // Run a small number of test tasks
    for _ in 0..4 {
        let collector_copy = Arc::clone(&collector);
        let config_copy = Arc::clone(&config);
        let done_copy = send.clone();
        tokio::spawn(async move {
            let mut sender = new_sender(config_copy, collector_copy, false);
            sender.do_loop().await;
            done_copy.send(()).unwrap();
        });
    }

    // Keep the time short
    tokio::time::sleep(TEST_DURATION).await;
    collector.stop();
    for _ in 0..4 {
        recv.recv().await.unwrap();
    }
    target.stop();

    // Expect reasonable results. This will be wrong if everything errors.
    let results = collector.get_results(start, SystemTime::now());
    assert!(results.attempts > 0);
    assert_eq!(results.successes, results.attempts);
    assert_eq!(results.failures, 0);
    assert!(results.throughput > 0.0);
    assert!(results.latency_avg > 0.0);
    assert!(results.latency_pct[0] <= results.latency_pct[100]);
    assert!(results.bytes_sent > 0);
    assert!(results.bytes_received > 0);
}

async fn make_target(use_tls: bool) -> Target {
    httptarget::Builder::new()
        .port(0)
        .use_localhost(true)
        .self_signed(use_tls)
        .build()
        .await
        .expect("Failed to create target")
}
