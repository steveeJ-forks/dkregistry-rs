//! Client library for Docker Registry API v2.
//!
//! This module provides a `Client` which can be used to list
//! images and tags, to check for the presence of blobs (manifests,
//! layers and other objects) by digest, and to retrieve them.
//!
//! ## Example
//!
//! ```rust,no_run
//! # extern crate dkregistry;
//! # extern crate tokio;
//! # fn main() {
//! # fn run() -> dkregistry::errors::Result<()> {
//! #
//! use tokio::runtime::current_thread::Runtime;
//! use dkregistry::v2::Client;
//!
//! // Retrieve an image manifest.
//! let mut runtime = Runtime::new()?;
//! let dclient = Client::configure()
//!                      .registry("quay.io")
//!                      .build()?;
//! let fetch = dclient.get_manifest("coreos/etcd", "v3.1.0");
//! let manifest = runtime.block_on(fetch)?;
//! #
//! # Ok(())
//! # };
//! # run().unwrap();
//! # }
//! ```

use super::errors::*;
use futures::prelude::*;
use reqwest::StatusCode;
use serde_json;

mod config;
pub use self::config::Config;

mod catalog;
// TODO: do we need a special stream type for this?
// pub use self::catalog::StreamCatalog;

mod auth;
pub use self::auth::{FutureTokenAuth, TokenAuth};

pub mod manifest;

mod tags;
pub use self::tags::StreamTags;

mod blobs;
pub use self::blobs::FutureBlob;

mod content_digest;
pub(crate) use self::content_digest::ContentDigest;

/// A Client to make outgoing API requests to a registry.
#[derive(Clone, Debug)]
pub struct Client {
    base_url: String,
    credentials: Option<(String, String)>,
    index: String,
    user_agent: Option<String>,
    token: Option<String>,
}

impl Client {
    pub fn configure() -> Config {
        Config::default()
    }

    /// Ensure remote registry supports v2 API.
    pub async fn ensure_v2_registry(self) -> Result<Self> {
        if !self.is_v2_supported().await? {
            bail!("remote server does not support docker-registry v2 API")
        };

        Ok(self)
    }

    /// Check whether remote registry supports v2 API.
    pub async fn is_v2_supported(&self) -> Result<bool> {
        let api_header = "Docker-Distribution-API-Version";
        let api_version = "registry/2.0";

        // GET request to bare v2 endpoint.
        let v2_endpoint = format!("{}/v2/", self.base_url);
        let url = reqwest::Url::parse(&v2_endpoint)
            .map_err(|e| Error::from(format!("failed to parse url string '{}'", &v2_endpoint)))?;

        trace!("GET {:?}", url);

        let get_v2 = self
            .build_reqwest(reqwest::Client::new().get(url))
            .send()
            .map_err(Error::from)
            .await;

        // Check status code and API headers according to spec:
        // https://docs.docker.com/registry/spec/api/#api-version-check
        get_v2
            .and_then(move |r| match (r.status(), r.headers().get(api_header)) {
                (StatusCode::OK, Some(x)) => Ok(x == api_version),
                (StatusCode::UNAUTHORIZED, Some(x)) => Ok(x == api_version),
                (s, v) => {
                    trace!("Got unexpected status {}, header version {:?}", s, v);
                    Ok(false)
                }
            })
            .map(|b| {
                trace!("v2 API supported: {}", &b);
                b
            })
    }

    /// Takes reqwest's async RequestBuilder and injects an authentication header if a token is present
    fn build_reqwest(
        &self,
        req_builder: reqwest::r#async::RequestBuilder,
    ) -> reqwest::r#async::RequestBuilder {
        let mut builder = req_builder;

        if let Some(token) = &self.token {
            builder = builder.header(reqwest::header::AUTHORIZATION, format!("Bearer {}", token))
        }
        if let Some(ua) = &self.user_agent {
            builder = builder.header(reqwest::header::USER_AGENT, ua.as_str());
        };

        builder
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct ApiError {
    code: String,
    message: String,
    detail: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct Errors {
    errors: Vec<ApiError>,
}
