mod error;

use openssl::{
    asn1::{Asn1Integer, Asn1Time},
    bn::BigNum,
    ec::{EcGroup, EcKey},
    hash::MessageDigest,
    nid::Nid,
    pkey::{PKey, Private},
    x509::{
        extension::{BasicConstraints, ExtendedKeyUsage, KeyUsage, SubjectAlternativeName},
        X509Name, X509,
    },
};
use std::{
    borrow::Cow,
    time::{Duration, SystemTime},
};

pub use error::Error;

const ONE_DAY: Duration = Duration::from_secs(24 * 60 * 60);

pub fn new_certificate(valid_days: u32) -> Result<(EcKey<Private>, X509), Error> {
    let now = SystemTime::now();
    let expiration = now.checked_add(ONE_DAY * valid_days).unwrap();
    let serial = Asn1Integer::from_bn(&BigNum::from_u32(1).unwrap())?;

    let mut name_builder = X509Name::builder()?;
    name_builder.append_entry_by_nid(Nid::COMMONNAME, "localhost")?;
    name_builder.append_entry_by_nid(Nid::ORGANIZATIONNAME, "apib")?;
    let name = name_builder.build();

    let ec_group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1)?;
    let ec_key = EcKey::generate(&ec_group)?;
    let key = PKey::from_ec_key(ec_key.clone())?;

    let mut builder = X509::builder()?;
    let not_before_time = to_unix_time(&now)?;
    builder.set_not_before(&not_before_time)?;
    let not_after_time = to_unix_time(&expiration)?;
    builder.set_not_after(&not_after_time)?;
    builder.set_serial_number(&serial)?;
    builder.set_subject_name(&name)?;
    builder.set_pubkey(&key)?;
    builder.append_extension(
        KeyUsage::new()
            .key_agreement()
            .data_encipherment()
            .digital_signature()
            .build()?,
    )?;
    builder.append_extension(BasicConstraints::new().ca().build()?)?;
    builder.append_extension(ExtendedKeyUsage::new().server_auth().build()?)?;
    builder.append_extension(
        SubjectAlternativeName::new()
            .ip("127.0.0.1")
            .build(&builder.x509v3_context(None, None))?,
    )?;

    let digest = MessageDigest::from_nid(Nid::SHA256).unwrap();
    builder.sign(&key, digest)?;

    let cert = builder.build();
    Ok((ec_key, cert))
}

pub fn write_key(key: &EcKey<Private>) -> Result<String, Error> {
    let pem = key.private_key_to_pem()?;
    match String::from_utf8_lossy(&pem) {
        Cow::Borrowed(b) => Ok(b.to_string()),
        Cow::Owned(o) => Ok(o),
    }
}

pub fn write_certificate(cert: &X509) -> Result<String, Error> {
    let pem = cert.to_pem()?;
    match String::from_utf8_lossy(&pem) {
        Cow::Borrowed(b) => Ok(b.to_string()),
        Cow::Owned(o) => Ok(o),
    }
}

fn to_unix_time(t: &SystemTime) -> Result<Asn1Time, Error> {
    let unix_time = t
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .try_into()
        .unwrap();
    Ok(Asn1Time::from_unix(unix_time)?)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_basic() {
        new_certificate(1).expect("Error creating certificate");
    }
}
