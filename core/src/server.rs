use std::future::Future;
use std::io;
use tokio::sync::mpsc;

use crate::{
    proxy::{run_proxy, ProxyConfig, Toxics},
    signal::Stop,
    toxic::{StreamDirection, Toxic, ToxicEvent, ToxicEventKind, ToxicKind},
};

// TODO: maybe make initial_toxics a vec of serialized toxics?

/// Run the Noxious server
pub async fn run(initial_toxics: Vec<()>, shutdown: impl Future) -> io::Result<()> {
    let proxy_config = ProxyConfig {
        name: "mongo".to_owned(),
        listen: "127.0.0.1:27016".to_owned(),
        upstream: "127.0.0.1:27017".to_owned(),
        rand_seed: None,
    };

    let (stop, stopper) = Stop::new();

    let (event_tx, event_rx) = mpsc::channel::<ToxicEvent>(16);

    // tokio::spawn(async move {
    //     if let Err(err) = run_proxy(proxy_config, event_rx, initial_toxics, stop).await {
    //         println!("run proxy err");
    //         dbg!(err);
    //     }
    //     println!("proxy finished");
    // });

    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        println!(" -- firing event remove a toxic");
        let _ = event_tx
            .send(ToxicEvent::new(
                "mongo",
                StreamDirection::Downstream,
                "foo1",
                ToxicEventKind::ToxicRemove("foo1".to_owned()),
            ))
            .await;
    });

    shutdown.await;
    println!("shutdown received, sending stop signal");
    stopper.stop();
    Ok(())
}
