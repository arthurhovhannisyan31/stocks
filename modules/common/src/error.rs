use std::{ffi::c_int, io, net::SocketAddr, path::PathBuf};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
  #[error("IO Error")]
  Io(#[from] io::Error),
  #[error("Failed registering signal: {signal}")]
  SignalError {
    #[source]
    err: io::Error,
    signal: c_int,
  },
  #[error("Failed locating path: `{source_path:?}`")]
  NotFound {
    err: io::Error,
    source_path: PathBuf,
  },
  #[error("Failed binding to address: `{addr:?}`")]
  AddressBindError { addr: SocketAddr, err: io::Error },
  #[error("Failed configuring TcpListener")]
  TcpListenerError { err: io::Error },
  #[error("Failed configuring TcpStream")]
  TcpStreamError { err: io::Error },
  #[error("Failed configuring UdpSocket")]
  UdpSocketError { err: io::Error },
  #[error("Deserialization error")]
  DeserializationError { err: serde_json::error::Error },
  #[error(transparent)]
  OtherError(#[from] anyhow::Error),
}
