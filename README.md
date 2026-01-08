<div style="display: flex; flex-direction: column; justify-content: center; align-items: center;" align="center">
    <h1><code>stocks</code></h1>
    <h4>Built with <a href="https://rust-lang.org/">🦀</a></h4>
</div>


[![main](https://github.com/arthurhovhannisyan31/stocks/actions/workflows/code-validation.yml/badge.svg?branch=main)](https://github.com/arthurhovhannisyan31/stocks/actions/workflows/code-validation.yml)
[![main](https://github.com/arthurhovhannisyan31/stocks/actions/workflows/packages-validation.yml/badge.svg?branch=main)](https://github.com/arthurhovhannisyan31/stocks/actions/workflows/packages-validation.yml)

## Overview

This is a simple data streaming client-server application.
[Client](./modules/quote-client/README.md) sends `TCP Request` and accepts streamed data on `UDP Socket`.
[Server](./modules/quote-server/README.md) generates random stock quotes data and streams filtered data to client.
Server health check mechanism excludes inactive clients from data streming and client sends ping messages to server.

## Description

![img.png](./static/images/client-server-diagram.png)

## Usage

Please build the target and run server and clients using terminal.

```shell
quote-server
```

```shell
quote-client -f ./mocks/client-tickers.txt -s 127.0.0.1:8000 -S 8001 -c 127.0.0.1:8002 
```

## Stack

- [Rust](https://rust-lang.org/)
- [Tracing](https://crates.io/crates/tracing)
- [Serde](https://crates.io/crates/serde)

## Credits

Crate implemented as part of the [Yandex practicum](https://practicum.yandex.ru/) course.

## License

Licensed under either of at your option.

* Apache License, Version 2.0, [LICENSE-APACHE](./LICENSE_APACHE) or http://www.apache.org/licenses/LICENSE-2.0
* MIT license [LICENSE-MIT](./LICENSE_MIT) or http://opensource.org/licenses/MIT
