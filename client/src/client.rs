use std::collections::HashMap;

use crate::{
    error::{ApiErrorResponse, ClientError},
    proxy::Proxy,
};
use noxious::proxy::{ProxyConfig, ProxyWithToxics};
use reqwest::{Client as HttpClient, Response, StatusCode};

/// A client for Noxious and Toxiproxy
/// It follows the same naming conventions for the methods.
#[derive(Debug)]
pub struct Client {
    base_url: String,
}

// TODO: fix the error type
pub type Result<T> = std::result::Result<T, ClientError>;

impl Client {
    /// Create a new client
    ///
    /// Panics if the given url starts with `https://`.
    pub fn new(url: &str) -> Client {
        if url.starts_with("https://") {
            panic!("the toxiproxy client does not support https");
        }
        let base_url = if !url.starts_with("http://") {
            format!("http://{}", url)
        } else {
            url.to_owned()
        };
        Client { base_url }
    }

    /// Returns a proxy by name, if it already exists
    pub async fn proxy(&self, name: &str) -> Result<Proxy> {
        let res = HttpClient::new()
            .get(self.base_url.clone() + "/proxies/" + name)
            .send()
            .await?;
        if res.status().is_success() {
            Ok(Proxy::from_proxy_with_toxics(
                &self.base_url,
                res.json::<ProxyWithToxics>().await?,
            ))
        } else {
            Err(get_error_body(res, StatusCode::OK).await)
        }
    }

    /// Returns a map with all the proxies and their toxics
    pub async fn proxies(&self) -> Result<HashMap<String, Proxy>> {
        let res = HttpClient::new()
            .get(self.base_url.clone() + "/proxies")
            .send()
            .await?;
        if res.status().is_success() {
            Ok(res
                .json::<HashMap<String, ProxyWithToxics>>()
                .await?
                .into_iter()
                .map(|(name, proxy)| (name, Proxy::from_proxy_with_toxics(&self.base_url, proxy)))
                .collect())
        } else {
            Err(get_error_body(res, StatusCode::OK).await)
        }
    }

    /// Instantiates a new proxy config, sends it to the server
    /// The server starts listening on the specified address
    pub async fn create_proxy(&self, name: &str, listen: &str, upstream: &str) -> Result<Proxy> {
        let mut proxy = Proxy::new(&self.base_url, name, listen, upstream);
        proxy.save().await?;
        Ok(proxy)
    }

    /// Create a list of proxies using a configuration list. If a proxy already exists,
    /// it will be replaced with the specified configuration.
    pub async fn populate(&self, proxies: &[ProxyConfig]) -> Result<Vec<Proxy>> {
        let res = HttpClient::new()
            .post(self.base_url.clone() + "/populate")
            .json(proxies)
            .send()
            .await?;
        if res.status().is_success() {
            Ok(res
                .json::<Vec<ProxyWithToxics>>()
                .await?
                .into_iter()
                .map(|item| Proxy::from_proxy_with_toxics(&self.base_url, item))
                .collect::<Vec<Proxy>>())
        } else {
            Err(get_error_body(res, StatusCode::CREATED).await)
        }
    }

    /// Resets the state of all proxies by removing all the toxic from all proxies
    pub async fn reset_state(&self) -> Result<()> {
        let res = HttpClient::new()
            .post(self.base_url.clone() + "/reset")
            .send()
            .await?;
        if res.status().is_success() {
            Ok(())
        } else {
            Err(get_error_body(res, StatusCode::NO_CONTENT).await)
        }
    }
}

pub(crate) async fn get_error_body(res: Response, expected_status: StatusCode) -> ClientError {
    let code = res.status();
    if let Ok(api_error) = res.json::<ApiErrorResponse>().await {
        ClientError::ApiError(api_error)
    } else {
        ClientError::UnexpectedStatusCode(code.into(), expected_status.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn client_does_not_allow_https() {
        let _client = Client::new("https://blahblah");
    }

    #[test]
    fn client_adds_protocol() {
        let client = Client::new("blahblah");
        assert_eq!("http://blahblah", client.base_url);
    }
}
