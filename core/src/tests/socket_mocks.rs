use crate::socket::{Listener, Stream};
use async_trait::async_trait;
use mockall::{mock, predicate::*};
use std::{io, net::SocketAddr};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};

mock! {
    pub MemoryListener {}

    #[async_trait]
    impl Listener for MemoryListener {
        type S = MockMemoryStream;

        async fn bind(addr: &str) -> io::Result<Self>
        where
            Self: Sized;

        async fn accept(&self) -> io::Result<(MockMemoryStream, SocketAddr)>;
    }
}

mock! {
    pub MemoryStream {}

    #[async_trait]
    impl Stream for MemoryStream {
        async fn connect(addr: &str) -> io::Result<Self>
        where
            Self: Sized + 'static;

        fn into_split(self) -> (OwnedReadHalf, OwnedWriteHalf);
    }
}

// /// A simple wrapper around Tokio TcpListener to make it mockable
// #[derive(Debug)]
// pub struct SocketListener {
//     inner: TcpListener,
// }

// /// A simple wrapper around Tokio TcpStream to make it mockable
// #[derive(Debug)]
// pub struct SocketStream {
//     inner: TcpStream,
// }

// // #[cfg_attr(test, automock)]
// #[async_trait]
// impl Listener for SocketListener {
//     type S = SocketStream;
//     /// Creates a new SocketListener, which will be bound to the specified address.
//     async fn bind(addr: &str) -> io::Result<SocketListener>
//     where
//         Self: Sized,
//     {
//         Ok(SocketListener {
//             inner: TcpListener::bind(addr).await?,
//         })
//     }

//     /// Accepts a new incoming connection from this listener.
//     async fn accept(&self) -> io::Result<(Self::S, SocketAddr)> {
//         let (stream, addr) = self.inner.accept().await?;
//         let wrapper = SocketStream { inner: stream };
//         Ok((wrapper, addr))
//     }
// }
// #[async_trait]
// impl Stream for SocketStream {
//     /// Opens a TCP connection to a remote host.
//     async fn connect(addr: &str) -> io::Result<Self>
//     where
//         Self: Sized,
//     {
//         let inner = TcpStream::connect(addr).await?;
//         Ok(SocketStream { inner })
//     }

//     /// Splits the inner `TcpStream` into a read half and a write half, which
//     /// can be used to read and write the stream concurrently.
//     fn into_split(self) -> (OwnedReadHalf, OwnedWriteHalf) {
//         self.inner.into_split()
//     }
// }
