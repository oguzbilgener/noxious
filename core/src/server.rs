use std::future::Future;
use std::io;
use tokio::sync::mpsc;

use crate::{
    proxy::{run_proxy, ProxyConfig, ToxicEvent, ToxicEventKind, Toxics},
    signal::Stop,
    toxic::{StreamDirection, Toxic, ToxicKind},
};

// TODO: maybe make initial_toxics a vec of serialized toxics?

/// Run the Noxious server
pub async fn run(_initial_toxics: Vec<()>, shutdown: impl Future) -> io::Result<()> {
    let proxy_config = ProxyConfig {
        name: "mongo".to_owned(),
        listen: "127.0.0.1:27016".to_owned(),
        upstream: "127.0.0.1:27017".to_owned(),
    };

    let (stop, stopper) = Stop::new();

    let (event_tx, event_rx) = mpsc::channel::<ToxicEvent>(16);

    let toxic_one = Toxic {
        kind: ToxicKind::Noop,
        name: "foo".to_owned(),
        toxicity: 1.0,
        direction: StreamDirection::Downstream,
    };

    let toxic_two = Toxic {
        kind: ToxicKind::Noop,
        name: "foo2".to_owned(),
        toxicity: 1.0,
        direction: StreamDirection::Downstream,
    };

    let toxic_three = Toxic {
        kind: ToxicKind::Noop,
        name: "foo3".to_owned(),
        toxicity: 1.0,
        direction: StreamDirection::Downstream,
    };

    let toxic_up_one = Toxic {
        kind: ToxicKind::Noop,
        name: "up_1".to_owned(),
        toxicity: 1.0,
        direction: StreamDirection::Upstream,
    };

    let initial_toxics = Toxics {
        upstream: vec![toxic_up_one],
        downstream: vec![toxic_one, toxic_two, toxic_three],
    };

    tokio::spawn(async move {
        if let Err(err) = run_proxy(proxy_config, event_rx, initial_toxics, stop).await {
            println!("run proxy err");
            dbg!(err);
        }
        println!("proxy finished");
    });

    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    println!(" -- firing event remove a toxic");

    event_tx.send(ToxicEvent::new(
        "mongo",
        StreamDirection::Downstream,
        "foo",
        ToxicEventKind::ToxicRemove("foo".to_owned()),
    )).await;

    shutdown.await;
    println!("shutdown received, sending stop signal");
    stopper.stop();
    Ok(())
}
