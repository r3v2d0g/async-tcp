/**************************************************************************************************
 *                                                                                                *
 * This Source Code Form is subject to the terms of the Mozilla Public                            *
 * License, v. 2.0. If a copy of the MPL was not distributed with this                            *
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.                                       *
 *                                                                                                *
 **************************************************************************************************/

// =========================================== Imports ========================================== \\

#[cfg(feature = "async-io")]
mod async_io;

#[cfg(feature = "async-net")]
mod async_net;

use async_peek::AsyncPeek;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use futures_lite::{ready, AsyncRead, AsyncWrite, Stream};
use std::io::Result;
use std::net::{Shutdown, SocketAddr};

// ========================================= Interfaces ========================================= \\

/// A TCP socket server, listening for connections.
///
/// After creating a `TcpListener` by [`bind`ing] it to a socket address, it listens for incoming
/// TCP connections. Thse can be accepted by calling [`accept()`] or by awaiting items from the
/// stream of [`incoming`] connections.
///
/// Cloning a [`TcpListener`] creates another handle to the same socket. The socket will be closed
/// when all handles to it are dropped.
///
/// The Transmission Control Protocol is specified in [IETF RFC 793].
///
/// [`bind`ing]: TcpListener::bind()
/// [`accept()`]: TcpListener::accept()
/// [`incoming`]: TcpListener::incoming()
/// [IETF RFC 793]: https://tools.ietf.org/html/rfc793
pub trait TcpListener: Clone + Sized {
    /// The type of a TCP connection created by [`accept`ing] an incoming connection.
    ///
    /// [`accept`ing]: TcpListener::accept()
    type Stream: TcpStream;

    /// Creates a new `TcpListener` bound to the given address.
    ///
    /// Binding with a port number of `0` will request that the operating system assigns an
    /// available port to this listener. The assigned port can be queried via [`local_addr()`].
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # use async_net_ as async_net;
    /// use async_tcp::TcpListener;
    /// use std::net::SocketAddr;
    /// # use std::io::Result;
    /// use std::str::FromStr;
    ///
    /// # async fn tcp_bind() -> Result<impl TcpListener> {
    /// #
    /// let addr = SocketAddr::from_str("127.0.0.1:0").unwrap();
    /// let listener = async_net::TcpListener::bind(addr).await?;
    /// #
    /// # Ok(listener) }
    /// #
    /// # smol::block_on(tcp_bind()).unwrap();
    /// ```
    ///
    /// [`local_addr()`]: TcpListener::local_addr()
    fn bind(addr: SocketAddr) -> Bind<Self>;

    /// Returns the local socket address of this listener.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # smol::block_on(async {
    /// #
    /// # use async_net_ as async_net;
    /// use async_tcp::TcpListener;
    /// use std::net::SocketAddr;
    /// # use std::io::Result;
    /// use std::str::FromStr;
    ///
    /// let addr = SocketAddr::from_str("127.0.0.1:1105").unwrap();
    /// let listener = async_net::TcpListener::bind(addr).await?;
    ///
    /// # fn tcp_local_addr<Listener: TcpListener>(addr: SocketAddr, listener: Listener) -> Result<()> {
    /// #
    /// assert_eq!(addr, listener.local_addr()?);
    /// #
    /// # Ok(()) }
    /// #
    /// # tcp_local_addr(addr, listener)?;
    /// #
    /// # Result::<()>::Ok(()) });
    /// ```
    fn local_addr(&self) -> Result<SocketAddr>;

    /// Accepts a new incoming connection.
    ///
    /// Returns a [`TcpStream`] and the address it is connected to.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # smol::block_on(async {
    /// #
    /// # use async_net_ as async_net;
    /// use async_tcp::{TcpListener, TcpStream};
    /// use std::net::SocketAddr;
    /// # use std::io::Result;
    /// use std::str::FromStr;
    ///
    /// let addr = SocketAddr::from_str("127.0.0.1:0").unwrap();
    /// let listener = async_net::TcpListener::bind(addr).await?;
    ///
    /// # let addr = listener.local_addr()?;
    /// # let conn = smol::spawn(async_net::TcpStream::connect(addr));
    /// #
    /// # async fn tcp_accept<Listener: TcpListener>(
    /// #     listener: &Listener,
    /// # ) -> Result<impl TcpStream> {
    /// #
    /// let (stream, addr) = listener.accept().await?;
    /// assert_eq!(stream.peer_addr()?, addr);
    /// #
    /// # Ok(stream) }
    /// #
    /// # let stream = tcp_accept(&listener).await?;
    /// # let conn = conn.await?;
    /// # assert_eq!(conn.local_addr()?, stream.peer_addr()?);
    /// # assert_eq!(conn.peer_addr()?, stream.local_addr()?);
    /// #
    /// # Result::<()>::Ok(()) });
    /// ```
    fn accept(&self) -> Accept<Self> {
        Accept { listener: self }
    }

    /// Attempts to accept a new incoming connection from this listener.
    ///
    /// On success, returns a [`TcpStream`] and the address it is connected to.
    ///
    /// If no new incoming connection is ready to be accepted, the current task is registered to be
    /// notified when one becomes ready to be accepted or the socket closed, and `Poll::Pending` is
    /// returned.
    ///
    /// This method exists to be used by [`accept()`] and [`incoming()`], we recommend you use of
    /// these methods instead.
    ///
    /// [`accept()`]: TcpListener::accept()
    /// [`incoming()`]: TcpListener::incoming()
    fn poll_accept(&self, ctx: &mut Context) -> Poll<Result<(Self::Stream, SocketAddr)>>;

    /// Returns a stream of incoming connections.
    ///
    /// Iterating over this stream is equivalent to calling [`accept()`] in a loop. The stream of
    /// connections is infinite, ie. it will never return `None`.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # smol::block_on(async {
    /// #
    /// # use async_net_ as async_net;
    /// use async_tcp::{TcpListener, TcpStream};
    /// use futures_lite::StreamExt;
    /// use std::net::SocketAddr;
    /// # use std::io::Result;
    /// use std::str::FromStr;
    ///
    /// let addr = SocketAddr::from_str("127.0.0.1:0").unwrap();
    /// let listener = async_net::TcpListener::bind(addr).await?;
    /// #
    /// # let addr = listener.local_addr()?;
    /// # let conn = smol::spawn(async_net::TcpStream::connect(addr));
    /// #
    /// # async fn tcp_incoming<Listener: TcpListener>(
    /// #     listener: &Listener,
    /// # ) -> Result<impl TcpStream> {
    /// #
    /// let mut incoming = listener.incoming();
    ///
    /// while let Some(stream) = incoming.next().await {
    ///     let mut stream = stream?;
    ///     assert_eq!(stream.local_addr()?, listener.local_addr()?);
    ///     # return Ok(stream);
    /// }
    /// #
    /// # unreachable!(); }
    /// #
    /// # let stream = tcp_incoming(&listener).await?;
    /// # let conn = conn.await?;
    /// # assert_eq!(conn.local_addr()?, stream.peer_addr()?);
    /// # assert_eq!(conn.peer_addr()?, stream.local_addr()?);
    /// #
    /// # Result::<()>::Ok(()) });
    /// ```
    ///
    /// [`accept()`]: TcpListener::accept()
    fn incoming(&self) -> Incoming<Self> {
        Incoming { listener: self }
    }

    /// Gets the value of the `IP_TTL` option for this socket.
    ///
    /// For more information about this option, see [`set_ttl()`].
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # smol::block_on(async {
    /// #
    /// # use async_net_ as async_net;
    /// use async_tcp::TcpListener;
    /// use std::net::SocketAddr;
    /// # use std::io::Result;
    /// use std::str::FromStr;
    ///
    /// let addr = SocketAddr::from_str("127.0.0.1:0").unwrap();
    /// let listener = async_net::TcpListener::bind(addr).await.unwrap();
    ///
    /// # fn tcp_ttl<Listener: TcpListener>(listener: Listener) -> Result<()> {
    /// #
    /// listener.set_ttl(100)?;
    /// assert_eq!(listener.ttl()?, 100);
    /// #
    /// # Ok(()) }
    /// #
    /// # tcp_ttl(listener)?;
    /// #
    /// # Result::<()>::Ok(()) });
    /// ```
    ///
    /// [`set_ttl()`]: TcpListener::set_ttl()
    fn ttl(&self) -> Result<u32>;

    /// Sets the value for the `IP_TTL` option on this socket.
    ///
    /// This value sets the time-to-live field that is used in every packet sent from this socket.
    ///
    /// ## Example
    ///
    /// ```rust
    /// # smol::block_on(async {
    /// #
    /// # use async_net_ as async_net;
    /// use async_tcp::TcpListener;
    /// use std::net::SocketAddr;
    /// # use std::io::Result;
    /// use std::str::FromStr;
    ///
    /// let addr = SocketAddr::from_str("127.0.0.1:0").unwrap();
    /// let listener = async_net::TcpListener::bind(addr).await.unwrap();
    ///
    /// # fn tcp_set_ttl<Listener: TcpListener>(listener: Listener) -> Result<()> {
    /// #
    /// listener.set_ttl(100)?;
    /// assert_eq!(listener.ttl()?, 100);
    /// #
    /// # Ok(()) }
    /// #
    /// # tcp_set_ttl(listener)?;
    /// #
    /// # Result::<()>::Ok(()) });
    /// ```
    fn set_ttl(&self, ttl: u32) -> Result<()>;
}

pub trait TcpStream: AsyncPeek + AsyncRead + AsyncWrite + Sized {
    fn connect(addr: SocketAddr) -> Connect<Self>;

    fn local_addr(&self) -> Result<SocketAddr>;

    fn peer_addr(&self) -> Result<SocketAddr>;

    fn shutdown(&self, how: Shutdown) -> Result<()>;

    fn nodelay(&self) -> Result<bool>;

    fn set_nodelay(&self, nodelay: bool) -> Result<()>;

    fn ttl(&self) -> Result<u32>;

    fn set_ttl(&self, ttl: u32) -> Result<()>;
}

// ============================================ Types =========================================== \\

pub struct Bind<Listener> {
    fut: Pin<Box<dyn Future<Output = Result<Listener>> + Send>>,
}

pub struct Accept<'listener, Listener> {
    listener: &'listener Listener,
}

pub struct Incoming<'listener, Listener> {
    listener: &'listener Listener,
}

pub struct Connect<Stream> {
    fut: Pin<Box<dyn Future<Output = Result<Stream>> + Send>>
}

// ========================================== impl From ========================================= \\

impl<Listener> From<Pin<Box<dyn Future<Output = Result<Listener>> + Send>>> for Bind<Listener>
where
    Listener: TcpListener,
{
    fn from(fut: Pin<Box<dyn Future<Output = Result<Listener>> + Send>>) -> Self {
        Bind { fut }
    }
}

impl<Stream> From<Pin<Box<dyn Future<Output = Result<Stream>> + Send>>> for Connect<Stream>
where
    Stream: TcpStream,
{
    fn from(fut: Pin<Box<dyn Future<Output = Result<Stream>> + Send>>) -> Self {
        Connect { fut }
    }
}

// ========================================= impl Future ======================================== \\

impl<Listener: TcpListener> Future for Bind<Listener> {
    type Output = Result<Listener>;

    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        self.fut.as_mut().poll(ctx)
    }
}

impl<Listener: TcpListener> Future for Accept<'_, Listener> {
    type Output = Result<(Listener::Stream, SocketAddr)>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        self.listener.poll_accept(ctx)
    }
}

impl<Stream: TcpStream> Future for Connect<Stream> {
    type Output = Result<Stream>;

    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        self.fut.as_mut().poll(ctx)
    }
}

// ========================================= impl Stream ======================================== \\

impl<Listener: TcpListener> Stream for Incoming<'_, Listener> {
    type Item = Result<Listener::Stream>;

    fn poll_next(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Option<Self::Item>> {
        let (stream, _) = ready!(self.listener.poll_accept(ctx))?;
        Poll::Ready(Some(Ok(stream)))
    }
}
