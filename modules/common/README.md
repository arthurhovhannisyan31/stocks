<div style="display: flex; flex-direction: column; justify-content: center; align-items: center;" align="center">
    <h1><code>common</code></h1>
    <h4>Built with <a href="https://rust-lang.org/">🦀</a></h4>
</div>


[![main](https://github.com/arthurhovhannisyan31/stocks/actions/workflows/code-validation.yml/badge.svg?branch=main)](https://github.com/arthurhovhannisyan31/stocks/actions/workflows/code-validation.yml)
[![main](https://github.com/arthurhovhannisyan31/stocks/actions/workflows/packages-validation.yml/badge.svg?branch=main)](https://github.com/arthurhovhannisyan31/stocks/actions/workflows/packages-validation.yml)

## Overview

This is a common crate, which contains structures, types and functions used in workspace crates.

## Usage

Add the crate in your `Cargo.toml`

```toml
common = { path = "../common" }
```

Add module declaration in your crate `main.rs|lib.rs`.

```rust
use common::*;
```

...

## Stack

- [Rust](https://rust-lang.org/)
- [Serde](https://crates.io/crates/serde)