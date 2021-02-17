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
        name: "foo1".to_owned(),
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

    // let initial_toxics = Toxics {
    //     upstream: vec![toxic_up_one],
    //     downstream: vec![toxic_one, toxic_two, toxic_three],
    // };
    let initial_toxics = Toxics {
        upstream: Vec::new(),
        downstream: Vec::new(),
    };

    let s2 = stop.clone();

    tokio::spawn(async move {
        let proxy_config = ProxyConfig {
            name: "echo".to_owned(),
            listen: "127.0.0.1:12344".to_owned(),
            upstream: "127.0.0.1:12345".to_owned(),
        };
        let (_, event_rx) = mpsc::channel::<ToxicEvent>(16);
        if let Err(err) = run_proxy(
            proxy_config,
            event_rx,
            Toxics {
                upstream: Vec::new(),
                downstream: Vec::new(),
            },
            s2,
        )
        .await
        {
            println!("run proxy err");
            dbg!(err);
        }
        println!("proxy finished");
    });

    tokio::spawn(async move {
        if let Err(err) = run_proxy(proxy_config, event_rx, initial_toxics, stop).await {
            println!("run proxy err");
            dbg!(err);
        }
        println!("proxy finished");
    });

    tokio::spawn(async move {
        let mut x = 0;
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
        loop {
            interval.tick().await;
            println!(" -- firing event remove a toxic");
            // let _ = event_tx
            //     .send(ToxicEvent::new(
            //         "echo",
            //         StreamDirection::Downstream,
            //         &format!("foo{}", (x + 1)),
            //         ToxicEventKind::ToxicRemove("foo".to_owned()),
            //     ))
            //     .await;
            x = (x + 1) % 3;
        }
    });

    shutdown.await;
    println!("shutdown received, sending stop signal");
    stopper.stop();
    Ok(())
}
