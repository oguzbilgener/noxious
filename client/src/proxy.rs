use crate::client::get_error_body;
use crate::client::Result;
use noxious::{
    proxy::{ProxyConfig, ProxyWithToxics},
    toxic::{StreamDirection, Toxic, ToxicKind},
};
use reqwest::{Client as HttpClient, StatusCode};

/// A proxy object returned by the [`Client`](Client).
/// To manipulate this proxy and manipulate the toxics, you can call methods on
/// this object.
#[derive(Debug, Clone, PartialEq)]
pub struct Proxy {
    base_url: String,
    created: bool,
    /// Contains the proxy listen and upstream address, name. You can mutate them
    /// and call `.save()` to update the proxy.
    pub config: ProxyConfig,
    toxics: Vec<Toxic>,
}

impl Proxy {
    /// Save saves changes to a proxy such as its enabled status or upstream port.
    /// Note: this does not update the toxics
    pub async fn save(&mut self) -> Result<()> {
        let request = if self.created {
            HttpClient::new()
                .post(self.base_url.clone() + "/proxies/" + &self.config.name)
                .json(&self.config)
        } else {
            HttpClient::new()
                .post(self.base_url.clone() + "/proxies")
                .json(&self.config)
        };
        let response = request.send().await?;
        if response.status().is_success() {
            self.created = true;
            Ok(())
        } else {
            let expected_status = if self.created {
                StatusCode::OK
            } else {
                StatusCode::CREATED
            };
            Err(get_error_body(response, expected_status).await)
        }
    }

    /// Enable a proxy again after it has been disabled
    pub async fn enable(&mut self) -> Result<()> {
        self.config.enabled = true;
        self.save().await
    }

    /// Disable a proxy so that no connections can pass through. This will drop all active connections.
    pub async fn disable(&mut self) -> Result<()> {
        self.config.enabled = false;
        self.save().await
    }

    /// Returns whether this proxy is enabled or not
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Give this proxy a new name, save it.
    pub async fn change_name(&mut self, new_name: &str) -> Result<()> {
        let old_name = self.config.name.clone();
        self.config.name = new_name.to_owned();
        let res = HttpClient::new()
            .post(self.base_url.clone() + "/proxies/" + &old_name)
            .json(&self.config)
            .send()
            .await?;
        if res.status().is_success() {
            Ok(())
        } else {
            Err(get_error_body(res, StatusCode::OK).await)
        }
    }

    /// Delete a proxy complete and close all existing connections through it. All information about
    /// the proxy such as listen port and active toxics will be deleted as well. If you just wish to
    /// stop and later enable a proxy, use `enable()` and `disable()`.
    pub async fn delete(self) -> Result<()> {
        let res = HttpClient::new()
            .delete(self.base_url.clone() + "/proxies/" + &self.config.name)
            .send()
            .await?;
        if res.status().is_success() {
            Ok(())
        } else {
            Err(get_error_body(res, StatusCode::NO_CONTENT).await)
        }
    }

    /// Returns a map of all active toxics and their attributes
    pub async fn toxics(&self) -> Result<Vec<Toxic>> {
        let res = HttpClient::new()
            .get(self.base_url.clone() + "/proxies/" + &self.config.name + "/toxics")
            .send()
            .await?;

        if res.status().is_success() {
            Ok(res.json::<Vec<Toxic>>().await?)
        } else {
            Err(get_error_body(res, StatusCode::OK).await)
        }
    }

    /// Add a new toxic to this proxy.

    /// # Example
    /// ```ignore
    /// use noxious_client::{Client, Toxic, ToxicKind, StreamDirection};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let toxic = Toxic {
    ///         kind: ToxicKind::Latency { latency: 40, jitter: 5 },
    ///         name: "myProxy_latency".to_owned(),
    ///         toxicity: 0.9,
    ///         direction: StreamDirection::Upstream,
    ///     };
    ///
    ///     let client = Client::new("127.0.0.1:8474");
    ///     let result = client.add_toxic(&toxic).await;
    /// }
    /// ```
    ///
    pub async fn add_toxic(&self, toxic: &Toxic) -> Result<Toxic> {
        let res = HttpClient::new()
            .post(self.base_url.clone() + "/proxies/" + &self.config.name + "/toxics")
            .json(toxic)
            .send()
            .await?;

        if res.status().is_success() {
            Ok(res.json::<Toxic>().await?)
        } else {
            Err(get_error_body(res, StatusCode::OK).await)
        }
    }

    /// Updates a toxic with the given name
    /// If toxicity is below zero, it will be sent as 0
    pub async fn update_toxic(
        &self,
        name: &str,
        toxicity: f32,
        kind: ToxicKind,
        direction: StreamDirection,
    ) -> Result<Toxic> {
        let toxicity: f32 = if toxicity < 0.0 { 0.0 } else { toxicity };
        let toxic = Toxic {
            kind,
            name: name.to_owned(),
            toxicity,
            direction,
        };
        let res = HttpClient::new()
            .post(self.base_url.clone() + "/proxies/" + &self.config.name + "/toxics/" + name)
            .json(&toxic)
            .send()
            .await?;

        if res.status().is_success() {
            Ok(res.json::<Toxic>().await?)
        } else {
            Err(get_error_body(res, StatusCode::OK).await)
        }
    }

    /// Removes a toxic with the given name
    pub async fn remove_toxic(&self, name: &str) -> Result<()> {
        let res = HttpClient::new()
            .delete(self.base_url.clone() + "/proxies/" + &self.config.name + "/toxics/" + name)
            .send()
            .await?;
        if res.status().is_success() {
            Ok(())
        } else {
            Err(get_error_body(res, StatusCode::NO_CONTENT).await)
        }
    }

    pub(crate) fn new(base_url: &str, name: &str, listen: &str, upstream: &str) -> Proxy {
        Proxy {
            base_url: base_url.to_owned(),
            created: false,
            config: ProxyConfig {
                name: name.to_owned(),
                listen: listen.to_owned(),
                upstream: upstream.to_owned(),
                enabled: true,
                rand_seed: None,
            },
            toxics: Vec::new(),
        }
    }

    #[doc(hidden)]
    pub fn from_proxy_with_toxics(base_url: &str, obj: ProxyWithToxics) -> Proxy {
        Proxy {
            base_url: base_url.to_owned(),
            created: true,
            config: obj.proxy,
            toxics: obj.toxics,
        }
    }
}
