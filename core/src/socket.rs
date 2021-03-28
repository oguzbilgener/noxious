use async_trait::async_trait;
#[cfg(test)]
use mockall::automock;
use pin_project_lite::pin_project;
use std::{io, net::SocketAddr};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::{TcpListener as TokioTcpListener, TcpStream as TokioTcpStream},
};

#[cfg(not(test))]
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};

/// The TcpListener interface we need to mock
#[cfg_attr(test, automock(type Stream=TcpStream;))]
#[async_trait]
pub trait SocketListener: Sized + Send + Sync {
    /// The associated listener interface to be mocked too
    type Stream: SocketStream + 'static;

    /// Creates a new SocketListener, which will be bound to the specified address.
    async fn bind(addr: &str) -> io::Result<Self>
    where
        Self: Sized;

    /// Accepts a new incoming connection from this listener.
    async fn accept(&self) -> io::Result<(Self::Stream, SocketAddr)>;
}

/// The TcpStream interface we need to mock
#[cfg_attr(test, automock)]
#[async_trait]
pub trait SocketStream: Sized + Send + Sync {
    /// Opens a TCP connection to a remote host.
    async fn connect(addr: &str) -> io::Result<Self>
    where
        Self: Sized + 'static;

    /// Splits the inner `TcpStream` into a read half and a write half, which
    /// can be used to read and write the stream concurrently.
    fn into_split(self) -> (ReadStream, WriteStream);
}

/// A simple wrapper around Tokio TcpListener to make it mockable
#[derive(Debug)]
pub struct TcpListener {
    inner: TokioTcpListener,
}

/// A simple wrapper around Tokio TcpStream to make it mockable
#[derive(Debug)]
pub struct TcpStream {
    inner: TokioTcpStream,
}

#[async_trait]
impl SocketListener for TcpListener {
    type Stream = TcpStream;

    async fn bind(addr: &str) -> io::Result<TcpListener>
    where
        Self: Sized,
    {
        Ok(TcpListener {
            inner: TokioTcpListener::bind(addr).await?,
        })
    }

    async fn accept(&self) -> io::Result<(Self::Stream, SocketAddr)> {
        let (stream, addr) = self.inner.accept().await?;
        let wrapper = TcpStream { inner: stream };
        Ok((wrapper, addr))
    }
}
#[async_trait]
impl SocketStream for TcpStream {
    async fn connect(addr: &str) -> io::Result<Self>
    where
        Self: Sized,
    {
        let inner = TokioTcpStream::connect(addr).await?;
        Ok(TcpStream { inner })
    }

    #[cfg(not(test))]
    fn into_split(self) -> (ReadStream, WriteStream) {
        let (read_half, write_half) = self.inner.into_split();
        (ReadStream::new(read_half), WriteStream::new(write_half))
    }

    #[cfg(test)]
    fn into_split(self) -> (ReadStream, WriteStream) {
        unimplemented!("must mock")
    }
}

#[cfg(not(test))]
type ReadHalf = OwnedReadHalf;
#[cfg(test)]
type ReadHalf = tokio_test::io::Mock;

#[cfg(not(test))]
type WriteHalf = OwnedWriteHalf;
#[cfg(test)]
type WriteHalf = tokio_test::io::Mock;

pin_project! {
    /// Wrapper for OwnedReadHalf for mocking
    #[derive(Debug)]
    pub struct ReadStream {
        #[pin]
        inner: ReadHalf,
    }
}

pin_project! {
    /// Wrapper for OwnedWriteHalf for mocking
    #[derive(Debug)]
    pub struct WriteStream {
        #[pin]
        inner: WriteHalf,

    }
}

#[cfg_attr(test, automock)]
impl ReadStream {
    pub(crate) fn new(inner: ReadHalf) -> ReadStream {
        ReadStream { inner }
    }
}

#[cfg_attr(test, automock)]
impl WriteStream {
    pub(crate) fn new(inner: WriteHalf) -> WriteStream {
        WriteStream { inner }
    }
}

impl AsyncRead for ReadStream {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        self.project().inner.poll_read(cx, buf)
    }
}

impl AsyncWrite for WriteStream {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, io::Error>> {
        self.project().inner.poll_write(cx, buf)
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), io::Error>> {
        self.project().inner.poll_flush(cx)
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), io::Error>> {
        self.project().inner.poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests {
    use tokio_test::assert_ok;

    use super::*;

    // Dummy test for coverage's sake
    #[tokio::test]
    async fn test_tcp_stream() {
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel::<()>();
        tokio::spawn(async move {
            let listener = TcpListener {
                inner: TokioTcpListener::bind("127.0.0.1:9909").await.unwrap(),
            };
            let _ = ready_tx.send(());
            let _ = listener.accept().await.unwrap();
        });

        assert_ok!(ready_rx.await);
        let _stream = TcpStream::connect("127.0.0.1:9909").await.unwrap();
        // let _ = stream.into_split();
    }
}
