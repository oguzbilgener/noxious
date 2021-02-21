use crate::{
    error::NotFoundError,
    link::Link,
    signal::Stop,
    state::ToxicStateHolder,
    stream::{Read, Write},
    toxic::{update_toxic_list_in_place, StreamDirection, Toxic, ToxicEvent},
};
use futures::{stream, StreamExt};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::{io, mem};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_util::codec::{BytesCodec, FramedRead, FramedWrite};

/// The default Go io.Copy buffer size is 32K, so also use 32K buffers here to imitate Toxiproxy.
const READ_BUFFER_SIZE: usize = 32768;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ProxyConfig {
    /// An arbitrary name
    pub name: String,
    /// The host name and the port the proxy listens on, like 127.0.0.1:5431
    pub listen: String,
    /// The host name and the port the proxy connects to, like 127.0.0:5432
    pub upstream: String,
    /// A random seed
    pub rand_seed: Option<u64>,
}

#[derive(Debug)]
pub struct Links {
    upstream: Link,
    client: Link,
    /// Optional, connection-wide state for toxics that need such state (like LimitData)
    /// Toxic Name -> State
    state_holder: Option<Arc<ToxicStateHolder>>,
}

#[derive(Debug, Clone)]
pub(super) struct Toxics {
    pub(super) upstream: Vec<Toxic>,
    pub(super) downstream: Vec<Toxic>,
}

#[derive(Debug)]
pub struct ProxyState {
    /// Socket address -> (Upstream, Downstream)
    clients: HashMap<SocketAddr, Links>,
    toxics: Toxics,
}

pub(crate) async fn run_proxy(
    config: ProxyConfig,
    receiver: mpsc::Receiver<ToxicEvent>,
    initial_toxics: Toxics,
    mut stop: Stop,
) -> io::Result<()> {
    let listener = TcpListener::bind(&config.listen).await?;
    println!("listening on port {}", &config.listen);

    let state = Arc::new(Mutex::new(ProxyState::new(initial_toxics.clone())));

    tokio::spawn(listen_toxic_events(
        state.clone(),
        receiver,
        stop.clone(),
        config.clone(),
    ));

    while !stop.stop_received() {
        let maybe_connection = tokio::select! {
            res = listener.accept() => Ok::<Option<(TcpStream, SocketAddr)>, io::Error>(Some(res?)),
            _ = stop.recv() => {
                Ok(None)
            },
        }?;

        if let Some((client_stream, addr)) = maybe_connection {
            println!("\n\n~~ new client connected at {}", addr);
            // TODO: wrap this error? (could not connect to upstream)
            let upstream = TcpStream::connect(&config.upstream).await?;

            let (client_read, client_write) = client_stream.into_split();
            let (upstream_read, upstream_write) = upstream.into_split();

            let client_read =
                FramedRead::with_capacity(client_read, BytesCodec::new(), READ_BUFFER_SIZE);
            let client_write = FramedWrite::new(client_write, BytesCodec::new());
            let upstream_read =
                FramedRead::with_capacity(upstream_read, BytesCodec::new(), READ_BUFFER_SIZE);
            let upstream_write = FramedWrite::new(upstream_write, BytesCodec::new());

            let toxics = state.lock().expect("ProxyState poisoned").toxics.clone();

            let res = create_links(
                state.clone(),
                addr,
                &config,
                &mut stop,
                toxics,
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
    client_read: Read,
    client_write: Write,
    upstream_read: Read,
    upstream_write: Write,
) -> io::Result<()> {
    let mut current_state = state.lock().expect(&format!(
        "ProxyState poisoned for upstream {}",
        addr.to_string()
    ));

    if current_state.clients.contains_key(&addr) {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "State error: there is already a client connected with this address: {}",
                addr.to_string()
            ),
        ));
    }

    let (links_stop, links_stopper) = stop.fork();

    let toxics_state_holder = ToxicStateHolder::for_toxics(&toxics);

    let mut upstream_link = Link::new(
        addr,
        StreamDirection::Upstream,
        config.clone(),
        links_stop.clone(),
    );
    let mut client_link = Link::new(
        addr,
        StreamDirection::Downstream,
        config.clone(),
        links_stop.clone(),
    );

    let upstream_handle = upstream_link.establish(
        client_read,
        upstream_write,
        toxics.upstream,
        toxics_state_holder.clone(),
    );
    let downstream_handle = client_link.establish(
        upstream_read,
        client_write,
        toxics.downstream,
        toxics_state_holder.clone(),
    );

    let addr = addr.clone();
    let state = state.clone();
    tokio::spawn(async move {
        // No need to listen for the stop signal here, we're ending as soon as one of the tasks have stopped.
        let _ = tokio::select! {
            up = upstream_handle => up,
            down = downstream_handle => down
        };
        links_stopper.stop();
        let mut state = state.lock().expect("ProxyState poisoned");
        state.clients.remove(&addr);
        println!("Removed {}", addr);
    });

    current_state.clients.insert(
        addr,
        Links {
            upstream: upstream_link,
            client: client_link,
            state_holder: toxics_state_holder,
        },
    );
    Ok(())
}

impl ProxyState {
    fn new(toxics: Toxics) -> Self {
        ProxyState {
            clients: HashMap::new(),
            toxics,
        }
    }
}

async fn listen_toxic_events(
    state: Arc<Mutex<ProxyState>>,
    mut receiver: mpsc::Receiver<ToxicEvent>,
    mut stop: Stop,
    config: ProxyConfig,
) {
    while !stop.stop_received() {
        let maybe_event: Option<ToxicEvent> = tokio::select! {
            res = receiver.recv() => res,
            _ = stop.recv() => None,
        };
        if let Some(event) = maybe_event {
            process_toxic_event(state.clone(), config.clone(), stop.clone(), event).await;
        } else {
            break;
        }
    }
}

async fn process_toxic_event(
    state: Arc<Mutex<ProxyState>>,
    config: ProxyConfig,
    stop: Stop,
    event: ToxicEvent,
) {
    let new_toxics = {
        let mut current_state = state.lock().expect("ProxyState poisoned");
        if let Err(err) = update_toxics(event, &mut current_state.toxics) {
            // TODO: log this, return a reply to event sender
            println!("toxic not found {}", err);
        }
        current_state.toxics.clone()
    };

    let old_map = {
        let mut current_state = state.lock().expect("ProxyState poisoned for upstream {}");
        mem::replace(&mut current_state.clients, HashMap::new())
    };

    let mut elements = stream::iter(old_map);
    while let Some((addr, links)) = elements.next().await {
        if let Err(err) = recreate_links(
            state.clone(),
            &config,
            stop.clone(),
            addr,
            links,
            new_toxics.clone(),
        )
        .await
        {
            // TODO: log this
            println!("Failed to recreate links for client {}, {:?}", addr, err);
        }
    }
}

async fn recreate_links(
    state: Arc<Mutex<ProxyState>>,
    config: &ProxyConfig,
    stop: Stop,
    addr: SocketAddr,
    links: Links,
    new_toxics: Toxics,
) -> io::Result<()> {
    let (client_read, upstream_write) = links.client.disband().await?;
    let (upstream_read, client_write) = links.upstream.disband().await?;
    create_links(
        state.clone(),
        addr,
        config,
        &mut stop.clone(),
        new_toxics,
        client_read,
        client_write,
        upstream_read,
        upstream_write,
    )
}

fn update_toxics(event: ToxicEvent, toxics: &mut Toxics) -> Result<(), NotFoundError> {
    match event.direction {
        StreamDirection::Upstream => {
            update_toxic_list_in_place(&mut toxics.upstream, event.kind)?;
        }
        StreamDirection::Downstream => {
            update_toxic_list_in_place(&mut toxics.downstream, event.kind)?;
        }
    }
    Ok(())
}
