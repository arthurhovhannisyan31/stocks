<div style="display: flex; flex-direction: column; justify-content: center; align-items: center;" align="center">
    <h1><code>quote client</code></h1>
    <h4>Built with <a href="https://rust-lang.org/">🦀</a></h4>
</div>

[![main](https://github.com/arthurhovhannisyan31/stocks/actions/workflows/code-validation.yml/badge.svg?branch=main)](https://github.com/arthurhovhannisyan31/stocks/actions/workflows/code-validation.yml)
[![main](https://github.com/arthurhovhannisyan31/stocks/actions/workflows/packages-validation.yml/badge.svg?branch=main)](https://github.com/arthurhovhannisyan31/stocks/actions/workflows/packages-validation.yml)

## Overview

This is the stock quote client crate which requests and receives streamed stock quotes data.

Each client requests a
specific list of tickers from server, which are listed in the provided file.
The file should have `txt` extension and tickers should be separated with a new line `\n`.

## Synopsis

- `-f, --tickers_file <PathBuf>` Path to tickers file
- `-s --server_tcp_addr <SocketAddr>` Server TCP address
- `-S --server_udp_port <u16>` Server UDP address port
- `-c --client_udp_addr <SocketAddr>` Client UDP address


- `--help`  Print help
- `-V, --version`  Print version

## Description

Request to server is sent using `TCP connection` and the response is read through same `TCP stream`.
A `UDP socket` is used to read server data and send `health check` messages on interval.
Client has `graceful shutdown` feature which listens
to [TERM_SIGNALS](https://docs.rs/signal-hook/latest/src/signal_hook/lib.rs.html#406) system signals.

## Usage

```shell
quote-client -f tickers.txt -s 127.0.0.1:8000 -S 8001 -c 127.0.0.1:8002 
```

## Stack

- [Rust](https://rust-lang.org/)
- [Clap](https://crates.io/crates/clap)
- [Serde](https://crates.io/crates/serde)
- [Signal hook](https://crates.io/crates/signal_hook)
- [Tracing](https://crates.io/crates/tracing)
