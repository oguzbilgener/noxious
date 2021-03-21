use std::collections::HashMap;

use noxious::proxy::ProxyWithToxics;
use noxious_client::{
    error::{ApiErrorResponse, ClientError},
    Client, Proxy, ProxyConfig, StreamDirection, Toxic, ToxicKind,
};
use tokio_test::assert_ok;
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn get_proxies_empty() {
    let mock_server = MockServer::start().await;
    let body = serde_json::to_value(&HashMap::<String, ProxyWithToxics>::new()).unwrap();
    Mock::given(method("GET"))
        .and(path("/proxies"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&mock_server)
        .await;

    let client = Client::new(&mock_server.uri());
    let result = client.proxies().await;
    assert_eq!(Ok(HashMap::new()), result);
}

#[tokio::test]
async fn get_proxy_not_found() {
    let mock_server = MockServer::start().await;
    let body = ApiErrorResponse {
        message: "proxy not found".to_owned(),
        status_code: 404,
    };

    Mock::given(method("GET"))
        .and(path("/proxies/blah"))
        .respond_with(ResponseTemplate::new(404).set_body_json(&body))
        .mount(&mock_server)
        .await;

    let client = Client::new(&mock_server.uri());
    let result = client.proxy("blah").await;
    assert_eq!(Err(ClientError::ApiError(body)), result);
}

#[tokio::test]
async fn get_proxy_found() {
    let mock_server = MockServer::start().await;
    let body = ProxyWithToxics {
        proxy: ProxyConfig {
            name: "blah".to_owned(),
            listen: "127.0.0.1:5555".to_owned(),
            upstream: "127.0.0.1:5556".to_owned(),
            enabled: true,
            rand_seed: None,
        },
        toxics: vec![Toxic {
            kind: ToxicKind::SlowClose { delay: 1000 },
            name: "t1".to_owned(),
            toxicity: 0.5,
            direction: StreamDirection::Upstream,
        }],
    };

    Mock::given(method("GET"))
        .and(path("/proxies/blah"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&mock_server)
        .await;

    let client = Client::new(&mock_server.uri());
    let result = client.proxy("blah").await;
    let proxy = Proxy::from_proxy_with_toxics(&mock_server.uri(), body);
    assert_eq!(Ok(proxy), result);
}

#[tokio::test]
async fn reset_state_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/reset"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    let client = Client::new(&mock_server.uri());
    let result = client.reset_state().await;
    assert_eq!(Ok(()), result);
}

#[tokio::test]
async fn populate() {
    let mock_server = MockServer::start().await;

    let config1 = ProxyConfig {
        name: "blah".to_owned(),
        listen: "127.0.0.1:5555".to_owned(),
        upstream: "127.0.0.1:5556".to_owned(),
        enabled: true,
        rand_seed: None,
    };
    let config2 = ProxyConfig {
        name: "p2".to_owned(),
        listen: "127.0.0.1:5553".to_owned(),
        upstream: "127.0.0.1:5554".to_owned(),
        enabled: true,
        rand_seed: None,
    };

    let p1 = ProxyWithToxics {
        proxy: config1.clone(),
        toxics: vec![Toxic {
            kind: ToxicKind::SlowClose { delay: 1000 },
            name: "t1".to_owned(),
            toxicity: 0.5,
            direction: StreamDirection::Upstream,
        }],
    };
    let p2 = ProxyWithToxics {
        proxy: config2.clone(),
        toxics: vec![Toxic {
            kind: ToxicKind::Latency {
                latency: 1000,
                jitter: 30,
            },
            name: "t1".to_owned(),
            toxicity: 1.0,
            direction: StreamDirection::Downstream,
        }],
    };
    let proxies = vec![p1, p2];
    let input = vec![config1, config2];

    Mock::given(method("POST"))
        .and(path("/populate"))
        .respond_with(ResponseTemplate::new(201).set_body_json(&proxies))
        .mount(&mock_server)
        .await;

    let client = Client::new(&mock_server.uri());
    let result = client.populate(&input).await;
    let expected = proxies
        .into_iter()
        .map(|p| Proxy::from_proxy_with_toxics(&mock_server.uri(), p))
        .collect();
    assert_eq!(Ok(expected), result);
}

#[tokio::test]
async fn populate_error() {
    let mock_server = MockServer::start().await;

    let body = ApiErrorResponse {
        message: "something odd happened".to_owned(),
        status_code: 500,
    };

    let input = vec![];

    Mock::given(method("POST"))
        .and(path("/populate"))
        .respond_with(ResponseTemplate::new(500).set_body_json(&body))
        .mount(&mock_server)
        .await;

    let client = Client::new(&mock_server.uri());
    let result = client.populate(&input).await;
    assert_eq!(Err(ClientError::ApiError(body)), result);
}

#[tokio::test]
async fn reset_state_error() {
    let mock_server = MockServer::start().await;

    let body = ApiErrorResponse {
        message: "something odd happened".to_owned(),
        status_code: 500,
    };

    Mock::given(method("POST"))
        .and(path("/reset"))
        .respond_with(ResponseTemplate::new(500).set_body_json(&body))
        .mount(&mock_server)
        .await;

    let client = Client::new(&mock_server.uri());
    let result = client.reset_state().await;
    assert_eq!(Err(ClientError::ApiError(body)), result);
}

#[tokio::test]
async fn reset_state_unexpected() {
    let mock_server = MockServer::start().await;

    let body = serde_json::json!({
        "thing": "asdf"
    });

    Mock::given(method("POST"))
        .and(path("/reset"))
        .respond_with(ResponseTemplate::new(500).set_body_json(&body))
        .mount(&mock_server)
        .await;

    let client = Client::new(&mock_server.uri());
    let result = client.reset_state().await;
    assert_eq!(Err(ClientError::UnexpectedStatusCode(500, 204)), result);
}

#[tokio::test]
async fn reset_state_io_error() {
    let client = Client::new("blah");
    let result = client.reset_state().await;
    match result {
        Err(ClientError::IoError(_)) => {}
        _ => {
            #[cfg_attr(tarpaulin, ignore)]
            panic!("invalid error kind")
        }
    }
}

#[tokio::test]
async fn populate_io_error() {
    let client = Client::new("blah");
    let result = client.populate(&Vec::new()).await;
    match result {
        Err(ClientError::IoError(_)) => {}
        _ => {
            #[cfg_attr(tarpaulin, ignore)]
            panic!("invalid error kind")
        }
    }
}

#[tokio::test]
async fn get_proxies_io_error() {
    let client = Client::new("blah");
    let result = client.proxies().await;
    match result {
        Err(ClientError::IoError(_)) => {}
        _ => {
            #[cfg_attr(tarpaulin, ignore)]
            panic!("invalid error kind")
        }
    }
}

#[tokio::test]
async fn get_proxy_io_error() {
    let client = Client::new("blah");
    let result = client.proxy("asdf").await;
    match result {
        Err(ClientError::IoError(_)) => {}
        _ => {
            #[cfg_attr(tarpaulin, ignore)]
            panic!("invalid error kind")
        }
    }
}

#[tokio::test]
async fn disable_proxy() {
    let mock_server = MockServer::start().await;
    let body = ProxyWithToxics {
        proxy: ProxyConfig {
            name: "blah".to_owned(),
            listen: "127.0.0.1:5555".to_owned(),
            upstream: "127.0.0.1:5556".to_owned(),
            enabled: true,
            rand_seed: None,
        },
        toxics: vec![Toxic {
            kind: ToxicKind::SlowClose { delay: 1000 },
            name: "t1".to_owned(),
            toxicity: 0.5,
            direction: StreamDirection::Upstream,
        }],
    };
    let mut body2 = body.clone();
    body2.proxy.enabled = false;
    let update_payload = &body2.proxy;

    Mock::given(method("GET"))
        .and(path("/proxies/blah"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/proxies/blah"))
        .and(body_json(update_payload))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&mock_server)
        .await;

    let client = Client::new(&mock_server.uri());
    let result = client.proxy("blah").await;
    let mut proxy = Proxy::from_proxy_with_toxics(&mock_server.uri(), body);
    assert_eq!(Ok(proxy.clone()), result);

    assert_ok!(proxy.disable().await);
    assert_eq!(false, proxy.is_enabled());
}

#[tokio::test]
async fn enable_proxy() {
    let mock_server = MockServer::start().await;
    let body = ProxyWithToxics {
        proxy: ProxyConfig {
            name: "blah".to_owned(),
            listen: "127.0.0.1:5555".to_owned(),
            upstream: "127.0.0.1:5556".to_owned(),
            enabled: false,
            rand_seed: None,
        },
        toxics: vec![Toxic {
            kind: ToxicKind::SlowClose { delay: 1000 },
            name: "t1".to_owned(),
            toxicity: 0.5,
            direction: StreamDirection::Upstream,
        }],
    };
    let mut body2 = body.clone();
    body2.proxy.enabled = true;
    let update_payload = &body2.proxy;

    Mock::given(method("GET"))
        .and(path("/proxies/blah"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/proxies/blah"))
        .and(body_json(update_payload))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&mock_server)
        .await;

    let client = Client::new(&mock_server.uri());
    let result = client.proxy("blah").await;
    let mut proxy = Proxy::from_proxy_with_toxics(&mock_server.uri(), body);
    assert_eq!(Ok(proxy.clone()), result);

    assert_ok!(proxy.enable().await);
    assert_eq!(true, proxy.is_enabled());
}

#[tokio::test]
async fn create() {
    let mock_server = MockServer::start().await;
    let body = ProxyWithToxics {
        proxy: ProxyConfig {
            name: "my_proxy".to_owned(),
            listen: "127.0.0.1:5556".to_owned(),
            upstream: "127.0.0.1:5557".to_owned(),
            enabled: true,
            rand_seed: None,
        },
        toxics: Vec::new(),
    };

    Mock::given(method("POST"))
        .and(path("/proxies"))
        .and(body_json(&body.proxy))
        .respond_with(ResponseTemplate::new(201).set_body_json(&body))
        .mount(&mock_server)
        .await;

    let client = Client::new(&mock_server.uri());
    let result = client
        .create_proxy("my_proxy", "127.0.0.1:5556", "127.0.0.1:5557")
        .await;
    let proxy = Proxy::from_proxy_with_toxics(&mock_server.uri(), body);
    assert_eq!(Ok(proxy), result);
}

#[tokio::test]
async fn create_error() {
    let mock_server = MockServer::start().await;

    let proxy = ProxyConfig {
        name: "my_proxy".to_owned(),
        listen: "127.0.0.1:5556".to_owned(),
        upstream: "127.0.0.1:5557".to_owned(),
        enabled: true,
        rand_seed: None,
    };

    let body = ApiErrorResponse {
        message: "something odd happened".to_owned(),
        status_code: 500,
    };

    Mock::given(method("POST"))
        .and(path("/proxies"))
        .and(body_json(&proxy))
        .respond_with(ResponseTemplate::new(500).set_body_json(&body))
        .mount(&mock_server)
        .await;

    let client = Client::new(&mock_server.uri());
    let result = client
        .create_proxy("my_proxy", "127.0.0.1:5556", "127.0.0.1:5557")
        .await;
    assert_eq!(Err(ClientError::ApiError(body)), result);
}

#[tokio::test]
async fn create_io_error() {
    let client = Client::new("asdf");
    let result = client
        .create_proxy("my_proxy", "127.0.0.1:5556", "127.0.0.1:5557")
        .await;
    match result {
        Err(ClientError::IoError(_)) => {}
        _ => {
            #[cfg_attr(tarpaulin, ignore)]
            panic!("invalid error kind")
        }
    }
}

#[tokio::test]
async fn change_proxy_name() {
    let mock_server = MockServer::start().await;
    let body = ProxyWithToxics {
        proxy: ProxyConfig {
            name: "my_proxy".to_owned(),
            listen: "127.0.0.1:5556".to_owned(),
            upstream: "127.0.0.1:5557".to_owned(),
            enabled: true,
            rand_seed: None,
        },
        toxics: Vec::new(),
    };
    let mut body2 = body.clone();
    body2.proxy.name = "updated_proxy".to_owned();

    Mock::given(method("POST"))
        .and(path("/proxies"))
        .and(body_json(&body.proxy))
        .respond_with(ResponseTemplate::new(201).set_body_json(&body))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/proxies/my_proxy"))
        .and(body_json(&body2.proxy))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body2))
        .mount(&mock_server)
        .await;

    let client = Client::new(&mock_server.uri());
    let result = client
        .create_proxy("my_proxy", "127.0.0.1:5556", "127.0.0.1:5557")
        .await;
    let mut proxy = Proxy::from_proxy_with_toxics(&mock_server.uri(), body);
    assert_eq!(Ok(proxy.clone()), result);

    let result = proxy.change_name("updated_proxy").await;
    assert_eq!(Ok(()), result);
}

#[tokio::test]
async fn get_toxics() {
    let mock_server = MockServer::start().await;
    let body = ProxyWithToxics {
        proxy: ProxyConfig {
            name: "my_proxy".to_owned(),
            listen: "127.0.0.1:5556".to_owned(),
            upstream: "127.0.0.1:5557".to_owned(),
            enabled: true,
            rand_seed: None,
        },
        toxics: vec![Toxic {
            kind: ToxicKind::SlowClose { delay: 1000 },
            name: "t1".to_owned(),
            toxicity: 0.5,
            direction: StreamDirection::Upstream,
        }],
    };

    Mock::given(method("GET"))
        .and(path("/proxies/my_proxy"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/proxies/my_proxy/toxics"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body.toxics))
        .mount(&mock_server)
        .await;

    let client = Client::new(&mock_server.uri());
    let result = client.proxy("my_proxy").await;
    let proxy = Proxy::from_proxy_with_toxics(&mock_server.uri(), body.clone());
    assert_eq!(Ok(proxy.clone()), result);

    let result = proxy.toxics().await;
    assert_eq!(Ok(body.toxics), result);
}

#[tokio::test]
async fn create_toxic() {
    let mock_server = MockServer::start().await;
    let body = ProxyWithToxics {
        proxy: ProxyConfig {
            name: "my_proxy".to_owned(),
            listen: "127.0.0.1:5556".to_owned(),
            upstream: "127.0.0.1:5557".to_owned(),
            enabled: true,
            rand_seed: None,
        },
        toxics: vec![Toxic {
            kind: ToxicKind::SlowClose { delay: 1000 },
            name: "t1".to_owned(),
            toxicity: 0.5,
            direction: StreamDirection::Upstream,
        }],
    };
    let mut toxic2 = body.toxics[0].clone();
    toxic2.name = "t2".to_owned();

    Mock::given(method("GET"))
        .and(path("/proxies/my_proxy"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/proxies/my_proxy/toxics"))
        .and(body_json(&toxic2))
        .respond_with(ResponseTemplate::new(200).set_body_json(&toxic2))
        .mount(&mock_server)
        .await;

    let client = Client::new(&mock_server.uri());
    let result = client.proxy("my_proxy").await;
    let proxy = Proxy::from_proxy_with_toxics(&mock_server.uri(), body.clone());
    assert_eq!(Ok(proxy.clone()), result);

    let result = proxy.add_toxic(&toxic2).await;
    assert_eq!(Ok(toxic2), result);
}
