use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use std::sync::Arc;

use tokio_rustls::rustls::internal::pemfile::{certs, pkcs8_private_keys, rsa_private_keys};
use tokio_rustls::rustls::{NoClientAuth, ServerConfig};
use tokio_rustls::TlsAcceptor;

use crate::error::Error;

pub(crate) fn new_tls_acceptor(
    cert_path: impl AsRef<Path>,
    key_path: impl AsRef<Path>,
) -> Result<TlsAcceptor, Error> {
    let mut config = ServerConfig::new(NoClientAuth::new());
    let certs = certs(&mut BufReader::new(
        File::open(cert_path.as_ref())
            .map_err(|e| crate::error_msg!("open cert file failed, err:{:?}", e))?,
    ))
    .map_err(|_| crate::error_msg!("invalid cert"))?;

    let mut key_vec = Vec::new();
    File::open(key_path.as_ref())?.read_to_end(&mut key_vec)?;

    let mut reader = BufReader::new(key_vec.as_slice());

    // try pkcs8 first
    let keys = match pkcs8_private_keys(&mut reader) {
        Ok(pkcs8) => pkcs8,
        Err(_e) => {
            let mut reader = BufReader::new(key_vec.as_slice());
            rsa_private_keys(&mut reader).map_err(|_| crate::error_msg!("invalid key"))?
        }
    };

    let key = keys
        .first()
        .ok_or_else(|| crate::error_msg!("invalid key, not data"))?;

    config
        .set_single_cert(certs, key.clone())
        .map_err(|e| crate::error_msg!("set tls cert failed, err: {:?}", e))?;

    Ok(TlsAcceptor::from(Arc::new(config)))
}
