# reqwest-hickory-resolver &emsp; [![Build Status]][actions] [![Latest Version]][crates.io]

[Build Status]: https://img.shields.io/github/actions/workflow/status/Xuanwo/reqwest-hickory-resolver/ci.yml
[actions]: https://github.com/Xuanwo/reqwest-hickory-resolver/actions?query=branch%3Amain
[Latest Version]: https://img.shields.io/crates/v/reqwest-hickory-resolver.svg
[crates.io]: https://crates.io/crates/reqwest-hickory-resolver

`reqwest-hickory-resolver` is the resolver for [reqwest](https://github.com/seanmonstar/reqwest) based on [`hickory-dns`](https://github.com/hickory-dns/hickory-dns).

## Quick Start

Init client with `HickoryResolver`.

```rust
use std::sync::Arc;

use reqwest::ClientBuilder;
use reqwest_hickory_resolver::HickoryResolver;

fn init_with_hickory_resolver() -> reqwest::Result<()> {
    let mut builder = ClientBuilder::new();
    builder = builder.dns_resolver(Arc::new(HickoryResolver::default()));
    builder.build()?;
    Ok(())
}
```


HickoryResolver has cache support, we can share the same resolver across different client
for better performance.

```rust
use std::sync::Arc;
use once_cell::sync::Lazy;
use reqwest::ClientBuilder;
use reqwest_hickory_resolver::HickoryResolver;

static GLOBAL_RESOLVER: Lazy<Arc<HickoryResolver>> =
    Lazy::new(|| Arc::new(HickoryResolver::default()));
    
fn init_with_hickory_resolver() -> reqwest::Result<()> {
    let mut builder = ClientBuilder::new();
    builder = builder.dns_resolver(GLOBAL_RESOLVER.clone());
    builder.build()?;
    Ok(())
}
```

## Contributing

Check out the [CONTRIBUTING.md](./CONTRIBUTING.md) guide for more details on getting started with contributing to this project.

## Getting help

Submit [issues](https://github.com/Xuanwo/reqwest-hickory-resolver/issues/new/choose) for bug report or asking questions in [discussion](https://github.com/Xuanwo/reqwest-hickory-resolver/discussions/new?category=q-a).

## Acknowledgements

This project is based on [reqwest](https://github.com/seanmonstar/reqwest)'s [`TrustDnsResolver`](https://github.com/seanmonstar/reqwest/blob/eeaece9709aa0dcb6b2b04b16d58ff5e580a6f40/src/dns/trust_dns.rs).

#### License

<sup>
Licensed under <a href="./LICENSE">Apache License, Version 2.0</a>.
</sup>
