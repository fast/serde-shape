# serde-shape

[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
[![MSRV 1.85][msrv-badge]](https://www.whatrustisit.com)
[![Apache 2.0 licensed][license-badge]][license-url]
[![Build Status][actions-badge]][actions-url]

[crates-badge]: https://img.shields.io/crates/v/serde-shape.svg
[crates-url]: https://crates.io/crates/serde-shape
[docs-badge]: https://img.shields.io/docsrs/serde-shape
[docs-url]: https://docs.rs/serde-shape
[msrv-badge]: https://img.shields.io/badge/MSRV-1.85-green?logo=rust
[license-badge]: https://img.shields.io/crates/l/serde-shape
[license-url]: LICENSE
[actions-badge]: https://github.com/fast/serde-shape/workflows/CI/badge.svg
[actions-url]: https://github.com/fast/serde-shape/actions?query=workflow%3ACI

`serde-shape` is a crate to reflect the shape of a Serde-derived type.

## Examples

Generate environment variable entries from a Serde-derived configuration shape:

```shell
cargo run -p serde-shape --example env_vars
```

## Minimum Rust version policy

This crate's minimum supported `rustc` version is `1.85.0`.

The current policy is that the minimum Rust version required to use this crate can be increased in minor version updates. For example, if `crate 1.0` requires Rust 1.85.0, then `crate 1.0.z` for all values of `z` will also require Rust 1.85.0 or newer. However, `crate 1.y` for `y > 0` may require a newer minimum version of Rust.

## License

This project is licensed under [Apache License, Version 2.0](LICENSE).
