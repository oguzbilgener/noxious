use crate::proxy::{ProxyConfig, ProxyRunner, Runner, Toxics};
use crate::signal::{Close, Stop};
use crate::socket::{ReadStream, WriteStream};
use crate::tests::socket_mocks::*;
use crate::toxic::{StreamDirection, Toxic, ToxicKind};
use lazy_static::lazy_static;
use mockall::predicate;
use std::{
    io,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::sync::Mutex as AsyncMutex;
use tokio_test::{assert_err, assert_ok, io as test_io};

lazy_static! {
    static ref MOCK_LOCK: AsyncMutex<()> = AsyncMutex::new(());
}

#[tokio::test]
async fn initialize_proxy_no_toxics_accept_fails() {
    let _lock = MOCK_LOCK.lock().await;
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
    let proxy = ProxyRunner::initialize_proxy::<MockMemoryListener>(config, toxics).await;
    assert_ok!(&proxy);
    let (listener, info) = proxy.unwrap();
    assert_eq!(expected_config, *info.config);

    let (_event_sender, event_receiver) = bmrng::channel(1);

    let (stop, _stopper) = Stop::new();
    let (_close, closer) = Close::new();

    let result =
        ProxyRunner::run_proxy::<MockMemoryListener>(listener, info, event_receiver, stop, closer)
            .await;
    assert_err!(&result);
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::Other,);
}

#[tokio::test]
async fn run_proxy_no_toxics_forward() {
    let _lock = MOCK_LOCK.lock().await;
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

    let listeners = Arc::new(Mutex::new(0));

    listener_ctx
        .expect()
        .with(predicate::eq(listen))
        .returning(move |c| {
            assert_eq!(listen, c);
            let listeners = listeners.clone();

            let mut listener = MockMemoryListener::default();
            listener.expect_accept().returning(move || {
                let mut val = listeners.lock().unwrap();
                // only accept one connection
                if *val > 0 {
                    return Err(io::Error::new(io::ErrorKind::ConnectionRefused, "done"));
                }
                *val += 1;
                let (client_read, mut client_handle_read) =
                    test_io::Builder::new().build_with_handle();
                let (client_write, mut client_handle_write) =
                    test_io::Builder::new().build_with_handle();

                client_handle_read.read(b"client writes");
                client_handle_write.write(b"upstream writes");

                let mut stream = MockMemoryStream::default();
                stream.expect_into_split().return_once_st(|| {
                    (ReadStream::new(client_read), WriteStream::new(client_write))
                });
                Ok((stream, SocketAddr::from(([127, 0, 0, 1], 29991))))
            });
            Ok(listener)
        });

    let upstream_ctx = MockMemoryStream::connect_context();
    let (upstream_read, mut upstream_handle_read) = test_io::Builder::new().build_with_handle();
    let (upstream_write, mut upstream_handle_write) = test_io::Builder::new().build_with_handle();

    upstream_handle_write.write(b"upstream writes");
    upstream_handle_read.read(b"client writes");
    // TODO: mock into_split with the mock stream instead of the real thing
    upstream_ctx
        .expect()
        .with(predicate::eq(upstream))
        .return_once(move |c| {
            assert_eq!(upstream, c);
            let mut stream = MockMemoryStream::default();

            stream.expect_into_split().return_once_st(|| {
                (
                    ReadStream::new(upstream_read),
                    WriteStream::new(upstream_write),
                )
            });
            Ok(stream)
        });

    let toxics = Toxics::noop();
    let proxy = ProxyRunner::initialize_proxy::<MockMemoryListener>(config, toxics).await;
    assert_ok!(&proxy);
    let (listener, info) = proxy.unwrap();
    assert_eq!(expected_config, *info.config);

    let (_event_sender, event_receiver) = bmrng::channel(1);

    let (stop, stopper) = Stop::new();
    let (close, closer) = Close::new();

    let handle = tokio::spawn(async move {
        let result = ProxyRunner::run_proxy(listener, info, event_receiver, stop, closer).await;
        assert_err!(result);
    });
    assert_ok!(handle.await);
    stopper.stop();
    let _ = close.recv().await;
}

#[tokio::test]
async fn run_proxy_with_slicer() {
    let _lock = MOCK_LOCK.lock().await;
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

    let listeners = Arc::new(Mutex::new(0));

    listener_ctx
        .expect()
        .with(predicate::eq(listen))
        .returning(move |c| {
            assert_eq!(listen, c);
            let listeners = listeners.clone();

            let mut listener = MockMemoryListener::default();
            listener.expect_accept().returning(move || {
                let mut val = listeners.lock().unwrap();
                // only accept one connection
                if *val > 0 {
                    return Err(io::Error::new(io::ErrorKind::ConnectionRefused, "done"));
                }
                *val += 1;
                let (client_read, mut client_handle_read) =
                    test_io::Builder::new().build_with_handle();
                let (client_write, mut client_handle_write) =
                    test_io::Builder::new().build_with_handle();

                client_handle_read.read(b"client writes");
                client_handle_write.write(b"upstream writes");

                let mut stream = MockMemoryStream::default();
                stream.expect_into_split().return_once_st(|| {
                    (ReadStream::new(client_read), WriteStream::new(client_write))
                });
                Ok((stream, SocketAddr::from(([127, 0, 0, 1], 29991))))
            });
            Ok(listener)
        });

    let upstream_ctx = MockMemoryStream::connect_context();
    let (upstream_read, mut upstream_handle_read) = test_io::Builder::new().build_with_handle();
    let (upstream_write, mut upstream_handle_write) = test_io::Builder::new().build_with_handle();

    upstream_handle_write.write(b"upstream writes");
    upstream_handle_read.read(b"client writes");
    // TODO: mock into_split with the mock stream instead of the real thing
    upstream_ctx
        .expect()
        .with(predicate::eq(upstream))
        .return_once(move |c| {
            assert_eq!(upstream, c);
            let mut stream = MockMemoryStream::default();

            stream.expect_into_split().return_once_st(|| {
                (
                    ReadStream::new(upstream_read),
                    WriteStream::new(upstream_write),
                )
            });
            Ok(stream)
        });

    let toxics = Toxics {
        upstream: vec![Toxic {
            name: "chop chop".to_owned(),
            kind: ToxicKind::Slicer {
                average_size: 12,
                size_variation: 4,
                delay: 0,
            },
            direction: StreamDirection::Upstream,
            toxicity: 1.0,
        }],
        downstream: Vec::new(),
    };
    let proxy = ProxyRunner::initialize_proxy::<MockMemoryListener>(config, toxics).await;
    assert_ok!(&proxy);
    let (listener, info) = proxy.unwrap();
    assert_eq!(expected_config, *info.config);

    let (_event_sender, event_receiver) = bmrng::channel(1);

    let (stop, stopper) = Stop::new();
    let (close, closer) = Close::new();

    let handle = tokio::spawn(async move {
        let result = ProxyRunner::run_proxy(listener, info, event_receiver, stop, closer).await;
        assert_err!(result);
    });
    assert_ok!(handle.await);
    stopper.stop();
    let _ = close.recv().await;
}
