use std::{fs::File, io::BufReader, path::Path};

use rustls::ServerConfig;
use rustls_pemfile::{certs, private_key};
use rustls_pki_types::{CertificateDer, PrivateKeyDer};

use crate::{Builder, Error};

pub(crate) fn make_server_config(builder: &Builder) -> Result<ServerConfig, Error> {
    assert!(builder.certificate.is_some());
    assert!(builder.key.is_some());
    let key = read_key(builder.key.as_ref().unwrap())?;
    let certs = read_certs(builder.certificate.as_ref().unwrap())?;

    let cfg = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;
    Ok(cfg)
}

fn read_key(key_file: &str) -> Result<PrivateKeyDer<'static>, Error> {
    let f = File::open(Path::new(key_file))?;
    let mut rdr = BufReader::new(f);
    match private_key(&mut rdr)? {
        Some(key) => Ok(key),
        None => Err(Error::Generic(format!("No key found in {}", key_file))),
    }
}

fn read_certs(cert_file: &str) -> Result<Vec<CertificateDer<'static>>, Error> {
    let f = File::open(cert_file)?;
    let mut rdr = BufReader::new(f);
    let mut result = vec![];
    for cert_result in certs(&mut rdr) {
        result.push(cert_result?);
    }
    Ok(result)
}
