use apib::{Builder, Collector, SendWrapper};
use httptarget::Target;
use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::sync::mpsc;

const TEST_DURATION: Duration = Duration::from_millis(500);

#[tokio::test]
async fn test_get() {
    let mut target = make_target().await;
    let address = target.address();
    let config = Arc::new(
        Builder::new()
            .set_url(&format!("http://127.0.0.1:{}/hello", address.port()))
            .build()
            .await
            .expect("Error building config"),
    );
    let mut sender = SendWrapper::new(config, false);
    sender.send().await.expect("Expected no error");
    target.stop();
}

#[tokio::test]
async fn test_get_http2_forced() {
    let mut target = make_target().await;
    let address = target.address();
    let config = Arc::new(
        Builder::new()
            .set_url(&format!("http://127.0.0.1:{}/hello", address.port()))
            .set_http2(true)
            .build()
            .await
            .expect("Error building config"),
    );
    let mut sender = SendWrapper::new(config, true);
    sender.send().await.expect("Expected no error");
    target.stop();
}

#[tokio::test]
async fn test_post() {
    let mut target = make_target().await;
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
    let mut sender = SendWrapper::new(config, false);
    sender.send().await.expect("Expected no error");
    target.stop();
}

#[tokio::test]
async fn test_not_found() {
    let mut target = make_target().await;
    let address = target.address();
    let config = Arc::new(
        Builder::new()
            .set_url(&format!("http://127.0.0.1:{}/NOTFOUND", address.port()))
            .build()
            .await
            .expect("Error building config"),
    );
    let mut sender = SendWrapper::new(config, false);
    assert!(sender.send().await.is_err());
    target.stop();
}

#[tokio::test]
async fn test_loops() {
    let mut target = make_target().await;
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
            let mut sender = SendWrapper::new(config_copy, false);
            sender.do_loop(collector_copy.as_ref()).await;
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
}

async fn make_target() -> Target {
    httptarget::Builder::new()
        .port(0)
        .use_localhost(true)
        .build()
        .await
        .expect("Failed to create target")
}
