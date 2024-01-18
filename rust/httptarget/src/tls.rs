use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use rustls::ServerConfig;
use rustls_pemfile::{certs, private_key};
use rustls_pki_types::{CertificateDer, PrivateKeyDer};

use crate::{Builder, Error};

const CERTIFICATE_VALID_DAYS: u32 = 7;

pub(crate) fn make_server_config(builder: &Builder) -> Result<ServerConfig, Error> {
    let (key, cert) = if builder.self_signed {
        let (key_pem, cert_pem) = makecert::new_certificate_strings(CERTIFICATE_VALID_DAYS)?;
        let key = read_key_string(&key_pem)?;
        let cert = read_cert_string(&cert_pem)?;
        (key, cert)
    } else if builder.certificate.is_some() && builder.key.is_some() {
        let key = read_key_file(builder.key.as_ref().unwrap())?;
        let cert = read_cert_file(builder.certificate.as_ref().unwrap())?;
        (key, cert)
    } else {
        return Err(Error::Generic("Incomplete TLS configuration".to_string()));
    };

    Ok(ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert, key)?)
}

fn read_key_string(data: &str) -> Result<PrivateKeyDer<'static>, Error> {
    let slice = data.as_bytes();
    let mut rdr = BufReader::new(slice);
    read_key(&mut rdr)
}

fn read_key_file(key_file: &str) -> Result<PrivateKeyDer<'static>, Error> {
    let f = File::open(Path::new(key_file))?;
    let mut rdr = BufReader::new(f);
    read_key(&mut rdr)
}

fn read_key(rdr: &mut dyn BufRead) -> Result<PrivateKeyDer<'static>, Error> {
    match private_key(rdr)? {
        Some(key) => Ok(key),
        None => Err(Error::Generic("No key found".to_string())),
    }
}

fn read_cert_string(data: &str) -> Result<Vec<CertificateDer<'static>>, Error> {
    let slice = data.as_bytes();
    let mut rdr = BufReader::new(slice);
    read_cert(&mut rdr)
}

fn read_cert_file(cert_file: &str) -> Result<Vec<CertificateDer<'static>>, Error> {
    let f = File::open(cert_file)?;
    let mut rdr = BufReader::new(f);
    read_cert(&mut rdr)
}

fn read_cert(rdr: &mut dyn BufRead) -> Result<Vec<CertificateDer<'static>>, Error> {
    let mut result = vec![];
    for cert_result in certs(rdr) {
        result.push(cert_result?);
    }
    Ok(result)
}
