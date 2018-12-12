use hyper::client;
use v2::*;

/// Configuration for a `Client`.
#[derive(Debug)]
pub struct Config {
    config: client::Client<hyper_rustls::HttpsConnector<client::HttpConnector>, hyper::Body>,
    handle: reactor::Handle,
    index: String,
    insecure_registry: bool,
    user_agent: Option<String>,
    username: Option<String>,
    password: Option<String>,
}

impl Config {
    /// Initialize `Config` with default values.
    pub fn default(handle: &reactor::Handle) -> Self {
        let dns_threads = 4;
        Self {
            config: hyper::client::Client::builder()
                .build(hyper_rustls::HttpsConnector::new(dns_threads)),
            handle: handle.clone(),
            index: "registry-1.docker.io".into(),
            insecure_registry: false,
            user_agent: Some(::USER_AGENT.to_owned()),
            username: None,
            password: None,
        }
    }

    /// Set registry service to use (vhost or IP).
    pub fn registry(mut self, reg: &str) -> Self {
        self.index = reg.to_owned();
        self
    }

    /// Whether to use an insecure HTTP connection to the registry.
    pub fn insecure_registry(mut self, insecure: bool) -> Self {
        self.insecure_registry = insecure;
        self
    }

    /// Set the user-agent to be used for registry authentication.
    pub fn user_agent(mut self, user_agent: Option<String>) -> Self {
        self.user_agent = user_agent;
        self
    }

    /// Set the username to be used for registry authentication.
    pub fn username(mut self, user: Option<String>) -> Self {
        self.username = user;
        self
    }

    /// Set the password to be used for registry authentication.
    pub fn password(mut self, password: Option<String>) -> Self {
        self.password = password;
        self
    }

    /// Read credentials from a JSON config file
    pub fn read_credentials<T: ::std::io::Read>(mut self, reader: T) -> Self {
        if let Ok(creds) = ::get_credentials(reader, &self.index) {
            self.username = creds.0;
            self.password = creds.1;
        };
        self
    }

    /// Return a `Client` to interact with a v2 registry.
    pub fn build(self) -> Result<Client> {
        let base = if self.insecure_registry {
            "http://".to_string() + &self.index
        } else {
            "https://".to_string() + &self.index
        };
        trace!(
            "Built client for {:?}: endpoint {:?} - user {:?}",
            self.index,
            base,
            self.username
        );
        let creds = match (self.username, self.password) {
            (None, None) => None,
            (u, p) => Some((
                u.unwrap_or_else(|| "".into()),
                p.unwrap_or_else(|| "".into()),
            )),
        };
        let c = Client {
            base_url: base,
            credentials: creds,
            hclient: self.config,
            index: self.index,
            user_agent: self.user_agent,
            token: None,
        };
        Ok(c)
    }
}
