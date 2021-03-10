use crate::socket::{SocketListener, SocketStream, ReadStream, WriteStream};
use async_trait::async_trait;
use mockall::{mock, predicate::*};
use std::{io, net::SocketAddr};

mock! {
    pub MemoryListener {}

    #[async_trait]
    impl SocketListener for MemoryListener {
        type Stream = MockMemoryStream;

        async fn bind(addr: &str) -> io::Result<Self>
        where
            Self: Sized;

        async fn accept(&self) -> io::Result<(MockMemoryStream, SocketAddr)>;
    }
}

mock! {
    pub MemoryStream {}

    #[async_trait]
    impl SocketStream for MemoryStream {
        async fn connect(addr: &str) -> io::Result<Self>
        where
            Self: Sized + 'static;

        fn into_split(self) -> (ReadStream, WriteStream);
    }
}
