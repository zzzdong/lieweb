use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use std::sync::Arc;

use rustls_pemfile::{certs, pkcs8_private_keys, rsa_private_keys};
use tokio_rustls::rustls::pki_types::PrivateKeyDer;
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::TlsAcceptor;

use crate::error::Error;

pub(crate) fn new_tls_acceptor(
    cert_path: impl AsRef<Path>,
    key_path: impl AsRef<Path>,
) -> Result<TlsAcceptor, Error> {
    let cert_chain = certs(&mut BufReader::new(
        File::open(cert_path.as_ref())
            .map_err(|e| crate::error_msg!("open cert file failed, err:{:?}", e))?,
    ))
    .collect::<Result<_, _>>()
    .map_err(|_| crate::error_msg!("invalid cert"))?;

    let mut key_bytes = Vec::new();
    File::open(key_path.as_ref())?.read_to_end(&mut key_bytes)?;

    let mut reader = BufReader::new(key_bytes.as_slice());

    // try pkcs8 first
    let key_der = match pkcs8_private_keys(&mut reader).next() {
        Some(Ok(pkcs8)) => PrivateKeyDer::Pkcs8(pkcs8),
        None => {
            let mut reader = BufReader::new(key_bytes.as_slice());
            let key = rsa_private_keys(&mut reader)
                .next()
                .ok_or(crate::error_msg!("invalid key"))?;

            PrivateKeyDer::Pkcs1(key.map_err(|_| crate::error_msg!("invalid key"))?)
        }
        _ => {
            return Err(crate::error_msg!("invalid key, not data"));
        }
    };

    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, key_der)?;

    Ok(TlsAcceptor::from(Arc::new(config)))
}
