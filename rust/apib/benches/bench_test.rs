use apib::{new_sender, Builder, Config};
use criterion::Criterion;
use httptarget::Target;
use std::sync::Arc;
use tokio::runtime::Runtime;

fn benchmarks(c: &mut Criterion) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Tokio failed to initialize");

    let mut target = runtime.block_on(async { make_target().await });
    let address = target.address();

    let config_http_get = runtime.block_on(async {
        Arc::new(
            Builder::new()
                .set_url(&format!("http://127.0.0.1:{}/hello", address.port()))
                .build()
                .await
                .expect("Error building config"),
        )
    });
    run_benchmark(c, &runtime, "GET http 1", false, config_http_get);

    let config_http_get_2 = runtime.block_on(async {
        Arc::new(
            Builder::new()
                .set_url(&format!("http://127.0.0.1:{}/hello", address.port()))
                .set_http2(true)
                .build()
                .await
                .expect("Error building config"),
        )
    });
    run_benchmark(c, &runtime, "GET http 2", true, config_http_get_2);

    let config_http_echo = runtime.block_on(async {
        Arc::new(
            Builder::new()
                .set_url(&format!("http://127.0.0.1:{}/echo", address.port()))
                .set_body_text("Hello, World!")
                .build()
                .await
                .expect("Error building config"),
        )
    });
    run_benchmark(c, &runtime, "POST http 1", false, config_http_echo);

    target.stop();
}

fn run_benchmark(
    c: &mut Criterion,
    runtime: &Runtime,
    name: &str,
    http2: bool,
    config: Arc<Config>,
) {
    c.bench_function(name, |b| {
        b.to_async(runtime).iter(|| {
            let config_copy = Arc::clone(&config);
            async move {
                let mut sender = new_sender(config_copy, http2);
                for _ in 0..1000 {
                    sender.send().await.expect("Expected no error");
                }
            }
        })
    });
}

criterion::criterion_group!(benches, benchmarks);
criterion::criterion_main!(benches);

async fn make_target() -> Target {
    httptarget::Builder::new()
        .port(0)
        .use_localhost(true)
        .build()
        .await
        .expect("Failed to create target")
}
