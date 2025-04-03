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

use hickory_resolver::config::ResolverConfig;
use hickory_resolver::name_server::TokioConnectionProvider;
use hickory_resolver::Resolver;
use hickory_resolver::TokioResolver;
use reqwest::dns::Addrs;
use reqwest::dns::Name;
use reqwest::dns::Resolve;
use reqwest::dns::Resolving;
use std::mem;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::OnceLock;

// Re-export ResolverOpts as part of the public API.
pub use hickory_resolver::config::ResolverOpts;

/// HickoryResolver implements reqwest [`Resolve`] so that we can use it as reqwest's DNS resolver.
#[derive(Debug, Default, Clone)]
pub struct HickoryResolver {
    /// Since we might not have been called in the context of a
    /// Tokio Runtime in initialization, so we must delay the actual
    /// construction of the resolver.
    state: Arc<OnceLock<TokioResolver>>,

    opts: Option<ResolverOpts>,
    rng: Option<rand::rngs::SmallRng>,
}

impl HickoryResolver {
    /// Configure the resolver with input options.
    pub fn with_options(mut self, opts: ResolverOpts) -> Self {
        self.opts = Some(opts);
        self
    }

    /// Enable shuffle for the hickory resolver to make sure the ip addrs returned are shuffled.
    ///
    /// NOTES: introduce shuffle will add extra overhead like more allocations and shuffling.
    pub fn with_shuffle(mut self, shuffle: bool) -> Self {
        if shuffle {
            use rand::SeedableRng;
            self.rng = Some(rand::rngs::SmallRng::from_os_rng());
        }

        self
    }

    /// Create a new resolver with the default configuration,
    /// which reads from `/etc/resolve.conf`.
    ///
    /// Fallback to default configuration if the system configuration fails.
    fn init_resolver(&self) -> TokioResolver {
        let mut builder =
            Resolver::builder(TokioConnectionProvider::default()).unwrap_or_else(|_| {
                Resolver::builder_with_config(
                    ResolverConfig::default(),
                    TokioConnectionProvider::default(),
                )
            });

        if let Some(mut opt) = self.opts.clone() {
            let _ = mem::replace(&mut builder.options_mut(), &mut opt);
        }

        builder.build()
    }
}

impl Resolve for HickoryResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let mut hickory_resolver = self.clone();
        Box::pin(async move {
            let resolver = hickory_resolver
                .state
                .get_or_init(|| hickory_resolver.init_resolver());

            let lookup = resolver.lookup_ip(name.as_str()).await?;

            let addrs: Addrs = if let Some(rng) = &mut hickory_resolver.rng {
                use rand::seq::SliceRandom;

                // Collect all the addresses into a vector and shuffle them.
                let mut ips = lookup.into_iter().collect::<Vec<_>>();
                ips.shuffle(rng);

                Box::new(ips.into_iter().map(|addr| SocketAddr::new(addr, 0)))
            } else {
                Box::new(lookup.into_iter().map(|addr| SocketAddr::new(addr, 0)))
            };

            Ok(addrs)
        })
    }
}
