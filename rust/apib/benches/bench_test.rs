use apib::{Builder, SendWrapper};
use criterion::Criterion;
use httptarget::Target;
use std::sync::Arc;

fn bench_get(c: &mut Criterion) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Tokio failed to initialize");

    let (config, mut target) = runtime.block_on(async {
        let target = make_target().await;
        let address = target.address();
        let config = Arc::new(
            Builder::new()
                .set_url(&format!("http://127.0.0.1:{}/hello", address.port()))
                .build()
                .await
                .expect("Error building config"),
        );
        (config, target)
    });

    c.bench_function("get hello 100", |b| {
        b.to_async(&runtime).iter(|| {
            let config_copy = Arc::clone(&config);
            async move {
                let mut sender = SendWrapper::new(config_copy, false);
                for _ in 0..10000 {
                    sender.send().await.expect("Expected no error");
                }
            }
        })
    });

    target.stop();
}

fn bench_echo(c: &mut Criterion) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Tokio failed to initialize");

    let (config, mut target) = runtime.block_on(async {
        let target = make_target().await;
        let address = target.address();
        let config = Arc::new(
            Builder::new()
                .set_url(&format!("http://127.0.0.1:{}/echo", address.port()))
                .set_body_text("Hello, World!")
                .build()
                .await
                .expect("Error building config"),
        );
        (config, target)
    });

    c.bench_function("post echo 100", |b| {
        b.to_async(&runtime).iter(|| {
            let config_copy = Arc::clone(&config);
            async move {
                let mut sender = SendWrapper::new(config_copy, false);
                for _ in 0..10000 {
                    sender.send().await.expect("Expected no error");
                }
            }
        })
    });

    target.stop();
}

criterion::criterion_group!(benches, bench_get, bench_echo);
criterion::criterion_main!(benches);

async fn make_target() -> Target {
    httptarget::Builder::new()
        .port(0)
        .use_localhost(true)
        .build()
        .await
        .expect("Failed to create target")
}
