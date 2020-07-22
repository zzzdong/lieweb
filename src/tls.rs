use std::fs::File;
use std::future::Future;
use std::io::{self, BufReader, Read};
use std::net::SocketAddr;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures::ready;
use hyper::server::accept::Accept;
use hyper::server::conn::{AddrIncoming, AddrStream};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_rustls::rustls::internal::pemfile::{certs, pkcs8_private_keys, rsa_private_keys};
use tokio_rustls::rustls::{NoClientAuth, ServerConfig};
use tokio_rustls::TlsAcceptor;

use crate::error::Error;

pub(crate) struct TlsIncoming {
    inner: AddrIncoming,
    tls_acceptor: TlsAcceptor,
}

impl TlsIncoming {
    pub(crate) fn new(
        addr: &SocketAddr,
        cert_path: impl AsRef<Path>,
        key_path: impl AsRef<Path>,
    ) -> Result<Self, Error> {
        let config = Self::build_config(cert_path, key_path)?;
        Self::new_with_config(addr, config)
    }

    pub(crate) fn new_with_config(addr: &SocketAddr, config: ServerConfig) -> Result<Self, Error> {
        let inner = AddrIncoming::bind(addr)?;
        let tls_acceptor = TlsAcceptor::from(Arc::new(config));

        Ok(TlsIncoming {
            inner,
            tls_acceptor,
        })
    }

    fn build_config(
        cert_path: impl AsRef<Path>,
        key_path: impl AsRef<Path>,
    ) -> Result<ServerConfig, Error> {
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

        Ok(config)
    }
}

impl Accept for TlsIncoming {
    type Conn = TlsStream;
    type Error = std::io::Error;

    fn poll_accept(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Self::Conn, Self::Error>>> {
        let mut_self = self.get_mut();
        match ready!(Pin::new(&mut mut_self.inner).poll_accept(cx)) {
            Some(Ok(sock)) => {
                let remote_addr = sock.remote_addr();
                let stream = mut_self.tls_acceptor.accept(sock);
                Poll::Ready(Some(Ok(TlsStream::new(stream, remote_addr))))
            }
            Some(Err(e)) => Poll::Ready(Some(Err(e))),
            None => Poll::Ready(None),
        }
    }
}

enum State {
    Handshaking(tokio_rustls::Accept<AddrStream>),
    Streaming(tokio_rustls::server::TlsStream<AddrStream>),
}

pub(crate) struct TlsStream {
    state: State,
    remote_addr: SocketAddr,
}

impl TlsStream {
    pub fn new(accpet: tokio_rustls::Accept<AddrStream>, remote_addr: SocketAddr) -> Self {
        TlsStream {
            remote_addr,
            state: State::Handshaking(accpet),
        }
    }

    pub fn remote_addr(&self) -> SocketAddr {
        self.remote_addr
    }
}

impl AsyncRead for TlsStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let pin = self.get_mut();
        match pin.state {
            State::Handshaking(ref mut fut) => match ready!(Pin::new(fut).poll(cx)) {
                Ok(mut stream) => {
                    let result = Pin::new(&mut stream).poll_read(cx, buf);
                    pin.state = State::Streaming(stream);
                    result
                }
                Err(e) => Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("tls handshake error: {:?}", e),
                ))),
            },
            State::Streaming(ref mut stream) => Pin::new(stream).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for TlsStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        let pin = self.get_mut();
        match pin.state {
            State::Handshaking(ref mut fut) => match ready!(Pin::new(fut).poll(cx)) {
                Ok(mut stream) => {
                    let result = Pin::new(&mut stream).poll_write(cx, buf);
                    pin.state = State::Streaming(stream);
                    result
                }
                Err(e) => Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("tls handshake error: {:?}", e),
                ))),
            },
            State::Streaming(ref mut stream) => Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        match self.state {
            State::Handshaking(_) => Poll::Ready(Ok(())),
            State::Streaming(ref mut stream) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        match self.state {
            State::Handshaking(_) => Poll::Ready(Ok(())),
            State::Streaming(ref mut stream) => Pin::new(stream).poll_shutdown(cx),
        }
    }
}
