use clap::Parser;
use std::{fs, path::Path, process};

#[derive(Parser, Debug)]
struct Args {
    #[arg(short)]
    cert_file: String,
    #[arg(short)]
    key_file: String,
    #[arg(short = 'd', default_value = "1")]
    valid_days: u32,
}

fn main() {
    let args = Args::parse();
    let (key, cert) = match makecert::new_certificate(args.valid_days) {
        Err(e) => {
            println!("Error making certificate: {}", e);
            process::exit(2);
        }
        Ok((key, cert)) => (key, cert),
    };

    let key_pem = match makecert::write_key(&key) {
        Ok(p) => p,
        Err(e) => {
            println!("Error writing PEM: {}", e);
            process::exit(3);
        }
    };
    let key_file_path = Path::new(&args.key_file);
    if let Err(e) = fs::write(key_file_path, key_pem) {
        println!("Error writing key file \"{}\": {}", args.key_file, e);
        process::exit(3);
    }

    let cert_pem = match makecert::write_certificate(&cert) {
        Ok(p) => p,
        Err(e) => {
            println!("Error writing PEM: {}", e);
            process::exit(3);
        }
    };
    let cert_file_path = Path::new(&args.cert_file);
    if let Err(e) = fs::write(cert_file_path, cert_pem) {
        println!("Error writing cert file \"{}\": {}", args.cert_file, e);
        process::exit(3);
    }
}
