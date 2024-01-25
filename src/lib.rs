//! This crate provides [`HickoryResolver`] that implements reqwest [`Resolve`] so that we
//! can use it as reqwest's DNS resolver.
//!
//! # Examples
//!
//! Init client with `HickoryResolver`.
//!
//! ```
//! use std::sync::Arc;
//!
//! use reqwest::ClientBuilder;
//! use reqwest_hickory_resolver::HickoryResolver;
//!
//! fn init_with_hickory_resolver() -> reqwest::Result<()> {
//!     let mut builder = ClientBuilder::new();
//!     builder = builder.dns_resolver(Arc::new(HickoryResolver::default()));
//!     builder.build()?;
//!     Ok(())
//! }
//! ```
//!
//! [`HickoryResolver`] has cache support, we can share the same resolver across different client
//! for better performance.
//!
//! ```
//! use std::sync::Arc;
//!
//! use once_cell::sync::Lazy;
//! use reqwest::ClientBuilder;
//! use reqwest_hickory_resolver::HickoryResolver;
//!
//! static GLOBAL_RESOLVER: Lazy<Arc<HickoryResolver>> =
//!     Lazy::new(|| Arc::new(HickoryResolver::default()));
//!
//! fn init_with_hickory_resolver() -> reqwest::Result<()> {
//!     let mut builder = ClientBuilder::new();
//!     builder = builder.dns_resolver(GLOBAL_RESOLVER.clone());
//!     builder.build()?;
//!     Ok(())
//! }
//! ```

use std::io;
use std::net::SocketAddr;
use std::sync::Arc;

pub use hickory_resolver::config::ResolverConfig;
pub use hickory_resolver::config::ResolverOpts;
use hickory_resolver::lookup_ip::LookupIpIntoIter;
use hickory_resolver::system_conf;
use hickory_resolver::TokioAsyncResolver;
use hyper::client::connect::dns::Name;
use once_cell::sync::OnceCell;
use reqwest::dns::Addrs;
use reqwest::dns::Resolve;
use reqwest::dns::Resolving;

/// HickoryResolver implements reqwest [`Resolve`] so that we can use it as reqwest's DNS resolver.
#[derive(Debug, Default, Clone)]
pub struct HickoryResolver {
    /// Since we might not have been called in the context of a
    /// Tokio Runtime in initialization, so we must delay the actual
    /// construction of the resolver.
    state: Arc<OnceCell<TokioAsyncResolver>>,
}

struct SocketAddrs {
    iter: LookupIpIntoIter,
}

impl Resolve for HickoryResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let resolver = self.clone();
        Box::pin(async move {
            let resolver = resolver.state.get_or_try_init(new_resolver)?;

            let lookup = resolver.lookup_ip(name.as_str()).await?;
            let addrs: Addrs = Box::new(SocketAddrs {
                iter: lookup.into_iter(),
            });
            Ok(addrs)
        })
    }
}

impl Iterator for SocketAddrs {
    type Item = SocketAddr;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|ip_addr| SocketAddr::new(ip_addr, 0))
    }
}

/// Create a new resolver with the default configuration,
/// which reads from `/etc/resolve.conf`.
fn new_resolver() -> io::Result<TokioAsyncResolver> {
    let (config, opts) = system_conf::read_system_conf().map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("error reading DNS system conf: {}", e),
        )
    })?;
    Ok(TokioAsyncResolver::tokio(config, opts))
}
