use crate::v2::*;
use reqwest::{self, StatusCode};

/// Convenience alias for future binary blob.
pub type FutureBlob = Box<dyn futures::Future<Output = Result<Vec<u8>>> + Send>;

impl Client {
    /// Check if a blob exists.
    pub async fn has_blob(&self, name: &str, digest: &str) -> Result<bool> {
        let url = {
            let ep = format!("{}/v2/{}/blobs/{}", self.base_url, name, digest);
            match reqwest::Url::parse(&ep) {
                Ok(url) => url,
                Err(e) => {
                    return Err(Error::from(format!(
                        "failed to parse url from string: {}",
                        e
                    )));
                }
            }
        };

        self.build_reqwest(reqwest::Client::new().head(url))
            .send()
            .await
            .and_then(|res| {
                trace!("Blob HEAD status: {:?}", res.status());
                match res.status() {
                    StatusCode::OK => Ok(true),
                    _ => Ok(false),
                }
            })
            .map_err(|e| format!("{}", e).into())
    }

    /// Retrieve blob.
    pub async fn get_blob(&self, name: &str, digest: &str) -> Result<Vec<u8>> {
        let digest = ContentDigest::try_new(digest.to_string())?;

        let blob = {
            let ep = format!("{}/v2/{}/blobs/{}", self.base_url, name, digest);
            let res = reqwest::Url::parse(&ep)
                .map_err(|e| {
                    crate::errors::Error::from(format!("failed to parse url from string: {}", e))
                })
                .map(|url| async {
                    self.build_reqwest(reqwest::Client::new().get(url))
                        .send()
                        .map_err(|e| crate::errors::Error::from(format!("{}", e)))
                        .await
                })?
                .await
                .and_then(|res| {
                    trace!("GET {} status: {}", res.url(), res.status());
                    let status = res.status();

                    if status.is_success()
                        // Let client errors through to populate them with the body
                        || status.is_client_error()
                    {
                        Ok(res)
                    } else {
                        Err(crate::errors::Error::from(format!(
                            "GET request failed with status '{}'",
                            status
                        )))
                    }
                })?;

            let body = res.bytes().await?;
            let len = body.len();
            let status = res.status();

            if status.is_success() {
                trace!("Successfully received blob with {} bytes ", len);
                body
            } else if status.is_client_error() {
                return Err(Error::from(format!(
                    "GET request failed with status '{}' and body of size {}: {:#?}",
                    status,
                    len,
                    String::from_utf8_lossy(&body)
                )));
            } else {
                // We only want to handle success and client errors here
                error!(
                        "Received unexpected HTTP status '{}' after fetching the body. Please submit a bug report.",
                        status
                    );
                return Err(Error::from(format!(
                    "GET request failed with status '{}'",
                    status
                )));
            }
        };

        digest.try_verify(&blob)?;

        Ok(blob.to_vec())
    }
}
