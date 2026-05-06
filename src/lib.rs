//! This crate provides [`HickoryResolver`] that implements reqwest [`Resolve`] so that you
//! can use it as reqwest's DNS resolver.
//!
//! # Examples
//!
//! Create a reqwest client with `HickoryResolver`.
//!
//! ```no_run
//! use std::sync::Arc;
//!
//! use reqwest::ClientBuilder;
//! use reqwest_hickory_resolver::HickoryResolverBuilder;
//!
//! fn create_client_with_hickory_resolver() -> reqwest::Client {
//!     let resolver = HickoryResolverBuilder::default().build().unwrap();
//!     ClientBuilder::new().dns_resolver(resolver).build().unwrap()
//! }
//! ```
//!
//! [`HickoryResolver`] has cache support, you can share the same resolver across different client
//! for better performance.

use hickory_resolver::Resolver;
use hickory_resolver::TokioResolver;
use hickory_resolver::net::runtime::TokioRuntimeProvider;
use rand::SeedableRng;
use rand::rngs::SmallRng;
use rand::rngs::SysRng;
use rand::seq::SliceRandom;
use reqwest::dns::Addrs;
use reqwest::dns::Name;
use reqwest::dns::Resolve;
use reqwest::dns::Resolving;
use std::net::SocketAddr;
use std::sync::Arc;

// Re-export ResolverOpts as part of the public API.
pub use hickory_resolver::config;
pub use hickory_resolver::config::ResolverConfig;
pub use hickory_resolver::config::ResolverOpts;

/// A builder for [`HickoryResolver`].
#[derive(Debug, Default, Clone)]
pub struct HickoryResolverBuilder {
    conf: Option<ResolverConfig>,
    opts: Option<ResolverOpts>,
    shuffle: bool,
}

impl HickoryResolverBuilder {
    /// Configure the resolver with the given options.
    pub fn with_options(mut self, options: ResolverOpts) -> Self {
        self.opts = Some(options);
        self
    }

    /// Configure the resolver with given config as a fallback if the system config cannot be used.
    ///
    /// This will use `/etc/resolv.conf` on Unix OSes and the registry on Windows.
    pub fn with_config(mut self, config: ResolverConfig) -> Self {
        self.conf = Some(config);
        self
    }

    /// Enable shuffle for the hickory resolver to make sure the ip addrs returned are shuffled.
    ///
    /// Note that introducing shuffle will add extra overhead like more allocations and shuffling.
    pub fn with_shuffle(mut self, shuffle: bool) -> Self {
        self.shuffle = shuffle;
        self
    }

    pub fn build(self) -> Result<HickoryResolver, Box<dyn std::error::Error + Send + Sync>> {
        let shuffler = if self.shuffle {
            Some(SmallRng::try_from_rng(&mut SysRng)?)
        } else {
            None
        };

        let builder = if let Ok(builder) = Resolver::builder(TokioRuntimeProvider::default()) {
            builder
        } else if let Some(conf) = self.conf {
            Resolver::builder_with_config(conf, TokioRuntimeProvider::default())
        } else {
            Resolver::builder_with_config(
                ResolverConfig::default(),
                TokioRuntimeProvider::default(),
            )
        };

        let resolver = if let Some(opts) = self.opts {
            builder.with_options(opts).build()?
        } else {
            builder.build()?
        };
        let resolver = Arc::new(resolver);

        Ok(HickoryResolver { resolver, shuffler })
    }
}

/// HickoryResolver implements reqwest [`Resolve`] so that you can use it as reqwest's DNS resolver.
#[derive(Debug, Clone)]
pub struct HickoryResolver {
    resolver: Arc<TokioResolver>,
    shuffler: Option<SmallRng>,
}

impl Resolve for HickoryResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let HickoryResolver {
            resolver,
            mut shuffler,
        } = self.clone();

        Box::pin(async move {
            let lookup = resolver.lookup_ip(name.as_str()).await?;
            let mut ips = lookup.iter().collect::<Vec<_>>();
            if let Some(shuffler) = shuffler.as_mut() {
                ips.shuffle(shuffler);
            }
            Ok(Box::new(ips.into_iter().map(|addr| SocketAddr::new(addr, 0))) as Addrs)
        })
    }
}
