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
pub async fn run(_initial_toxics: Vec<()>, shutdown: impl Future) -> io::Result<()> {
    let proxy_config = ProxyConfig {
        name: "mongo".to_owned(),
        listen: "127.0.0.1:27016".to_owned(),
        upstream: "127.0.0.1:27017".to_owned(),
        rand_seed: None,
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
        kind: ToxicKind::Latency {
            jitter: 0,
            latency: 10,
        },
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

    let s2 = stop.clone();

    tokio::spawn(async move {
        let proxy_config = ProxyConfig {
            name: "echo".to_owned(),
            listen: "127.0.0.1:12344".to_owned(),
            upstream: "127.0.0.1:12345".to_owned(),
            rand_seed: None,
        };
        let (_, event_rx) = mpsc::channel::<ToxicEvent>(16);
        if let Err(err) = run_proxy(
            proxy_config,
            event_rx,
            Toxics {
                upstream: vec![
                    /*Toxic {
                        kind: ToxicKind::Timeout { timeout: 5000 },
                        name: "timeout".to_owned(),
                        toxicity: 1.0,
                        direction: StreamDirection::Upstream,
                    }*/
                    Toxic {
                        kind: ToxicKind::SlowClose { delay: 8000 },
                        name: "scRe".to_owned(),
                        toxicity: 0.0,
                        direction: StreamDirection::Upstream,
                    },
                    Toxic {
                        kind: ToxicKind::LimitData { bytes: 12 },
                        name: "limit data".to_owned(),
                        toxicity: 0.3,
                        direction: StreamDirection::Upstream,
                    },
                    Toxic {
                        kind: ToxicKind::Latency {
                            latency: 500,
                            jitter: 450,
                        },
                        name: "lat!".to_owned(),
                        toxicity: 1.0,
                        direction: StreamDirection::Upstream,
                    },
                    Toxic {
                        kind: ToxicKind::SlowClose { delay: 2000 },
                        name: "sc".to_owned(),
                        toxicity: 1.0,
                        direction: StreamDirection::Upstream,
                    },
                    Toxic {
                        kind: ToxicKind::Noop,
                        name: "no".to_owned(),
                        toxicity: 1.0,
                        direction: StreamDirection::Upstream,
                    },
                ],
                downstream: vec![
                    Toxic {
                        kind: ToxicKind::Slicer {
                            average_size: 6,
                            size_variation: 2,
                            delay: 100,
                        },
                        name: "sl".to_owned(),
                        toxicity: 1.0,
                        direction: StreamDirection::Downstream,
                    },
                    Toxic {
                        kind: ToxicKind::SlowClose { delay: 5000 },
                        name: "sc2".to_owned(),
                        toxicity: 1.0,
                        direction: StreamDirection::Downstream,
                    },
                ],
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
