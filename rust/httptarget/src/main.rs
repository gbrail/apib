use clap::Parser;
use httptarget::Target;
use tokio::signal::unix::{signal, SignalKind};

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, default_value = "0")]
    port: u16,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let server = Target::new(args.port, false)
        .await
        .expect("Error listening on port");
    println!("Listening on {}", server.address());

    let mut hup = signal(SignalKind::hangup()).expect("Error making signal");
    let mut term = signal(SignalKind::terminate()).expect("Error making signal");
    tokio::select! {
        _ = hup.recv() => {}
        _ = term.recv() => {}
    };

    println!("Done.");
}
