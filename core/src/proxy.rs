use futures::{stream, StreamExt};
use std::io;
use std::mem;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::{borrow::BorrowMut, collections::HashMap};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::mpsc;
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpSocket, TcpStream},
};

use tokio_util::codec::{BytesCodec, FramedRead, FramedWrite};

use crate::error::NotFoundError;
use crate::link::Link;
use crate::signal::{spawn_stoppable, Stop};
use crate::toxic::{StreamDirection, Toxic};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ProxyConfig {
    /// An arbitrary name
    pub name: String,
    /// The host name and the port the proxy listens on, like 127.0.0.1:5431
    pub listen: String,
    /// The host name and the port the proxy connects to, like 127.0.0:5432
    pub upstream: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ToxicEventKind {
    ToxicAdd(Toxic),
    ToxicUpdate(Toxic),
    ToxicRemove(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToxicEvent {
    proxy_name: String,
    direction: StreamDirection,
    toxic_name: String,
    kind: ToxicEventKind,
}

#[derive(Debug)]
pub struct Links {
    upstream: Link,
    client: Link,
}

#[derive(Debug, Clone)]
pub(super) struct Toxics {
    pub(super) upstream: Vec<Toxic>,
    pub(super) downstream: Vec<Toxic>,
}

pub struct ProxyState {
    // Socket address --> (Upstream, Downstream)
    clients: HashMap<SocketAddr, Links>,
}

pub(crate) async fn run_proxy(
    config: ProxyConfig,
    receiver: mpsc::Receiver<ToxicEvent>,
    initial_toxics: Toxics,
    mut stop: Stop,
) -> io::Result<()> {
    let listener = TcpListener::bind(&config.listen).await?;
    println!("listening on port {}", &config.listen);

    let state = Arc::new(Mutex::new(ProxyState::new()));

    tokio::spawn(listen_toxic_events(
        state.clone(),
        receiver,
        stop.clone(),
        config.clone(),
    ));

    while !stop.is_stopped() {
        let maybe_connection = tokio::select! {
            res = listener.accept() => Ok::<Option<(TcpStream, SocketAddr)>, io::Error>(Some(res?)),
            _ = stop.recv() => {
                Ok(None)
            },
        }?;

        if let Some((client_stream, addr)) = maybe_connection {
            // TODO: wrap this error? (could not connect to upstream)
            let upstream = TcpStream::connect(&config.upstream).await?;

            let (client_read, client_write) = client_stream.into_split();
            let (upstream_read, upstream_write) = upstream.into_split();

            // TODO: the default Go io.Copy buffer size is 32K, so also use 32K buffers here to imitate Toxiproxy.
            let client_read = FramedRead::new(client_read, BytesCodec::new());
            let client_write = FramedWrite::new(client_write, BytesCodec::new());
            let upstream_read = FramedRead::new(upstream_read, BytesCodec::new());
            let upstream_write = FramedWrite::new(upstream_write, BytesCodec::new());

            let res = create_links(
                state.clone(),
                addr,
                &config,
                &mut stop,
                initial_toxics.clone(),
                client_read,
                client_write,
                upstream_read,
                upstream_write,
            );
            match res {
                Err(err) => {
                    // TODO: trace
                    println!("{}", err);
                    continue;
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn create_links(
    state: Arc<Mutex<ProxyState>>,
    addr: SocketAddr,
    config: &ProxyConfig,
    stop: &mut Stop,
    toxics: Toxics,
    client_read: FramedRead<OwnedReadHalf, BytesCodec>,
    client_write: FramedWrite<OwnedWriteHalf, BytesCodec>,
    upstream_read: FramedRead<OwnedReadHalf, BytesCodec>,
    upstream_write: FramedWrite<OwnedWriteHalf, BytesCodec>,
) -> io::Result<()> {
    let mut current_state = state.lock().expect(&format!(
        "ProxyState poisoned for upstream {}",
        addr.to_string()
    ));

    if current_state.clients.contains_key(&addr) {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "State error: there is already a client connected with this address, {}",
                addr.to_string()
            ),
        ));
    }

    let (links_stop, links_stopper) = stop.fork();

    let mut upstream_link = Link::new(
        addr,
        StreamDirection::Upstream,
        toxics.upstream,
        config.clone(),
        links_stop.clone(),
    );
    let mut client_link = Link::new(
        addr,
        StreamDirection::Downstream,
        toxics.downstream,
        config.clone(),
        links_stop.clone(),
    );

    let upstream_handle = upstream_link.establish(client_read, upstream_write);
    let downstream_handle = client_link.establish(upstream_read, client_write);

    let addr = addr.clone();
    let state = state.clone();
    spawn_stoppable(stop.clone(), async move {
        let some_handle = tokio::select! {
            up = upstream_handle => up,
            down = downstream_handle => down
        };
        println!("joined upstream and downstream {:?}", some_handle);
        links_stopper.stop();
        println!("removing {} from clients list as we disconnected", &addr);
        let mut state = state.lock().expect("ProxyState poisoned");
        state.clients.remove(&addr);
    });

    current_state.clients.insert(
        addr,
        Links {
            upstream: upstream_link,
            client: client_link,
        },
    );
    Ok(())
}

impl ProxyState {
    fn new() -> Self {
        ProxyState {
            clients: HashMap::new(),
        }
    }
}

async fn listen_toxic_events(
    state: Arc<Mutex<ProxyState>>,
    mut receiver: mpsc::Receiver<ToxicEvent>,
    mut stop: Stop,
    config: ProxyConfig,
) {
    while !stop.is_stopped() {
        let maybe_event: Option<ToxicEvent> = tokio::select! {
            res = receiver.recv() => res,
            _ = stop.recv() => None,
        };
        let stop = stop.clone();

        if let Some(event) = maybe_event {
            // Recreate all the links. The proxy may be unresponsive while updating

            let old_map = {
                let mut current_state = state.lock().expect("ProxyState poisoned for upstream {}");
                mem::replace(&mut current_state.clients, HashMap::new())
            };
            let direction = event.direction;
            let kind = event.kind.clone();

            let mut elements = stream::iter(old_map);
            while let Some((addr, links)) = elements.next().await {
                let (link_to_recreate, link_to_keep) = match event.direction {
                    StreamDirection::Upstream => (links.upstream, links.client),
                    StreamDirection::Downstream => (links.client, links.upstream),
                };
                let (reader, writer, old_toxics) = match link_to_recreate.disband().await {
                    Err(err) => {
                        // TODO: is this really an error
                        println!("disband recv err {:?}", err);
                        return;
                    }
                    Ok(res) => res,
                };

                if let Ok(updated_toxics) = build_toxic_list(old_toxics, kind.clone()) {
                    let mut updated_link = Link::new(
                        addr,
                        direction,
                        updated_toxics,
                        config.clone(),
                        stop.clone(),
                    );
                    // TODO: stop the proxy if we cannot establish link
                    updated_link.establish(reader, writer);

                    let links = if direction == StreamDirection::Upstream {
                        Links {
                            upstream: updated_link,
                            client: link_to_keep,
                        }
                    } else {
                        Links {
                            upstream: link_to_keep,
                            client: updated_link,
                        }
                    };

                    state
                        .lock()
                        .expect("ProxyState poisoned")
                        .clients
                        .insert(addr, links);
                } else {
                    // TODO: trace
                    println!(
                        "State error: Attempted to update or remove nonexistent toxic, for {} for address {}",
                        direction,
                        addr
                    );
                }
            }
        }
    }
}

fn build_toxic_list(
    mut toxics: Vec<Toxic>,
    event_kind: ToxicEventKind,
) -> Result<Vec<Toxic>, NotFoundError> {
    match event_kind {
        ToxicEventKind::ToxicAdd(toxic) => {
            toxics.push(toxic);
        }
        ToxicEventKind::ToxicUpdate(toxic) => {
            let old_toxic = toxics
                .iter_mut()
                .find(|el| el.get_name() == toxic.get_name())
                .ok_or(NotFoundError)?;
            let _ = mem::replace(old_toxic, toxic);
        }
        ToxicEventKind::ToxicRemove(toxic_name) => {
            let index = toxics
                .iter()
                .position(|el| el.get_name() == toxic_name)
                .ok_or(NotFoundError)?;
            toxics.remove(index);
        }
    }
    Ok(toxics)
}

impl ToxicEvent {
    pub fn new(
        proxy_name: &str,
        direction: StreamDirection,
        toxic_name: &str,
        kind: ToxicEventKind,
    ) -> Self {
        ToxicEvent {
            proxy_name: proxy_name.to_owned(),
            direction,
            toxic_name: toxic_name.to_owned(),
            kind,
        }
    }
}
