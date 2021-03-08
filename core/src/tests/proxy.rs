use crate::proxy::{self, ProxyConfig, Toxics};
use crate::signal::{Close, Stop};
use crate::socket::{Listener, Stream};
use crate::tests::socket_mocks::*;
use lazy_static::lazy_static;
use mock_io::tokio::{MockListener, MockStream};
use mockall::predicate;
use std::{io, net::SocketAddr};
use tokio::sync::Mutex as AsyncMutex;
use tokio_test::{assert_err, assert_ok, io as test_io};

lazy_static! {
    static ref MOCK_LOCK: AsyncMutex<()> = AsyncMutex::new(());
}

#[tokio::test]
async fn initialize_proxy_no_toxics_accept_fails() {
    MOCK_LOCK.lock().await;
    let listen = "127.0.0.1:5431";
    let config = ProxyConfig {
        name: "foo".to_owned(),
        listen: listen.to_owned(),
        upstream: "127.0.0.1:5432".to_owned(),
        enabled: true,
        rand_seed: None,
    };
    let expected_config = config.clone();
    let ctx = MockMemoryListener::bind_context();
    ctx.expect().with(predicate::eq(listen)).returning(|_c| {
        let mut m = MockMemoryListener::default();
        m.expect_accept()
            .returning(|| Err(io::Error::new(io::ErrorKind::Other, "oopsie")));
        Ok(m)
    });

    let toxics = Toxics::noop();
    let proxy = proxy::initialize_proxy::<MockMemoryListener>(config, toxics).await;
    assert_ok!(&proxy);
    let (listener, info) = proxy.unwrap();
    assert_eq!(expected_config, *info.config);

    let (_event_sender, event_receiver) = bmrng::channel(1);

    let (stop, _stopper) = Stop::new();
    let (_close, closer) = Close::new();

    let result = proxy::run_proxy(listener, info, event_receiver, stop, closer).await;
    assert_err!(&result);
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::Other,);
}

#[tokio::test]
async fn run_proxy_no_toxics_forward() {
    MOCK_LOCK.lock().await;
    let listen = "127.0.0.1:5431";
    let upstream = "127.0.0.1:5432";
    let config = ProxyConfig {
        name: "foo".to_owned(),
        listen: listen.to_owned(),
        upstream: upstream.to_owned(),
        enabled: true,
        rand_seed: None,
    };
    let expected_config = config.clone();
    let listener_ctx = MockMemoryListener::bind_context();
    listener_ctx
        .expect()
        .with(predicate::eq(listen))
        .returning(|_c| {
            let mut m = MockMemoryListener::default();
            m.expect_accept().return_once(move || {
                let mut stream = MockMemoryStream::default();
                Ok((stream, SocketAddr::from(([127, 0, 0, 1], 29991))))
            });
            Ok(m)
        });

    let (lis, handle) = MockListener::new();
    // lis.accept().await.unwrap().into_

    let upstream_ctx = MockMemoryStream::connect_context();
    let (mock, handle) = test_io::Builder::new().build_with_handle();
    // upstream_ctx
    //     .expect()
    //     .with(predicate::eq(upstream))
    //     .return_once(move |_c| {
    //         let mut stream = MockMemoryStream::default();

    //         stream.expect_into_split().return_once_st(|| {
    //             test_io
    //         })
    //         Ok(lis)
    //     });

    let toxics = Toxics::noop();
    let proxy = proxy::initialize_proxy::<MockMemoryListener>(config, toxics).await;
    assert_ok!(&proxy);
    let (listener, info) = proxy.unwrap();
    assert_eq!(expected_config, *info.config);

    let (_event_sender, event_receiver) = bmrng::channel(1);

    let (stop, _stopper) = Stop::new();
    let (_close, closer) = Close::new();

    tokio::spawn(async move {
        let result = proxy::run_proxy(listener, info, event_receiver, stop, closer).await;
        assert_ok!(result);
    });
}
