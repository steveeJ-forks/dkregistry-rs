use crate::errors::Result;
use crate::v2;
use serde_json;

#[derive(Debug, Default, Deserialize, Serialize)]
struct Catalog {
    pub repositories: Vec<String>,
}

impl v2::Client {
    pub async fn get_catalog(&self, paginate: Option<u32>) -> Result<Catalog> {
        let url = {
            let suffix = if let Some(n) = paginate {
                format!("?n={}", n)
            } else {
                "".to_string()
            };
            let ep = format!("{}/v2/_catalog{}", self.base_url.clone(), suffix);
            reqwest::Url::parse(&ep)
                .map_err(|e| format!("failed to parse url from string '{}': {}", ep, e))?
        };

        let res = self
            .build_reqwest(reqwest::Client::new().get(url))
            .send()
            .await?;

        let status = res.status();
        trace!("Got status: {:?}", status);

        if !status.is_success() {
            return Err(format!("get_catalog: wrong HTTP status '{}'", status).into());
        }

        let body = res.bytes().await?;

        let catalog: Catalog = serde_json::from_slice(&body)?;

        Ok(catalog)
    }
}
