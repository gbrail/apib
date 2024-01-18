use clap::Parser;
use httptarget::Builder;
use tokio::signal::unix::{signal, SignalKind};

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, default_value = "0")]
    port: u16,
    #[arg(short)]
    certificate: Option<String>,
    #[arg(short)]
    key: Option<String>,
    #[arg(short, default_value = "false")]
    self_signed_certs: bool,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let mut builder = Builder::new().port(args.port);
    if args.self_signed_certs {
        builder = builder.self_signed(true);
    }
    if let Some(cert) = args.certificate {
        builder = builder.certificate(&cert);
    }
    if let Some(key) = args.key {
        builder = builder.key(&key);
    }

    let server = builder.build().await.expect("Error listening on port");
    println!("Listening on {}", server.address());

    let mut interrupt = signal(SignalKind::interrupt()).expect("Error making signal");
    let mut term = signal(SignalKind::terminate()).expect("Error making signal");
    tokio::select! {
        _ = interrupt.recv() => {}
        _ = term.recv() => {}
    };

    println!("Done.");
}
