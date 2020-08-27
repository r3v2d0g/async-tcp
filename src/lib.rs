/**************************************************************************************************
 *                                                                                                *
 * This Source Code Form is subject to the terms of the Mozilla Public                            *
 * License, v. 2.0. If a copy of the MPL was not distributed with this                            *
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.                                       *
 *                                                                                                *
 **************************************************************************************************/

// ======================================== Documentation ======================================= \\

//! This crate provides [`TcpListener`] and [`TcpStream`], two traits that can be implemented by
//! any async versions of [`std::net::TcpListener`] and [`std::net::TcpStream`].

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
    /// # Result::<()>::Ok(()) }).unwrap();
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
    /// # Result::<()>::Ok(()) }).unwrap();
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
    /// # Result::<()>::Ok(()) }).unwrap();
    /// ```
    ///
    /// [`accept()`]: TcpListener::accept()
    fn incoming(&self) -> Incoming<Self> {
        Incoming { listener: self }
    }

    /// Gets the value of the `IP_TTL` option on this socket.
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
    /// # Result::<()>::Ok(()) }).unwrap();
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
    /// # Result::<()>::Ok(()) }).unwrap();
    /// ```
    fn set_ttl(&self, ttl: u32) -> Result<()>;
}

/// A TCP stream between a local and a remote socket.
///
/// A `TcpStream` can be created by either [`connect`ing] to an endpoint or [`accept`ing] a
/// connection on a [`TcpListener`].
///
/// [`TcpStream`] is a bidirectional stream that implements [`AsyncPeek`], [`AsyncRead`] and
/// [`AsyncWrite`].
///
/// Cloning a [`TcpStream`] creates another handle to the same socket. The socket will be closed
/// when all handles to it are dropped. The reading and writing portions of the connection can also
/// be shut down individually with the [`shutdown()`] method.
///
/// [`connect`ing]: TcpStream::connect()
/// [`accept`ing]: TcpListener::accept()
/// [`shutdown()`]: TcpStream::shutdown()
pub trait TcpStream: AsyncPeek + AsyncRead + AsyncWrite + Sized {
    /// Opens a TCP connection to the specified address.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # use async_net_ as async_net;
    /// # use async_tcp::TcpListener;
    /// use async_tcp::TcpStream;
    /// use std::net::SocketAddr;
    /// # use std::io::Result;
    /// use std::str::FromStr;
    ///
    /// # async fn tcp_connect() -> Result<impl TcpStream> {
    /// #
    /// # let addr = SocketAddr::from_str("127.0.0.1:0").unwrap();
    /// # let listener = async_net::TcpListener::bind(addr).await?;
    /// # let addr = listener.local_addr()?;
    /// // let addr = ...;
    /// #
    /// # let connect = smol::spawn(async move {
    /// let stream = async_net::TcpStream::connect(addr).await?;
    /// # Result::Ok(stream) });
    /// #
    /// # let (conn, peer_addr) = listener.accept().await?;
    /// # let stream = connect.await?;
    /// # assert_eq!(peer_addr, conn.peer_addr()?);
    /// # assert_eq!(peer_addr, stream.local_addr()?);
    /// assert_eq!(addr, stream.peer_addr()?);
    /// #
    /// # Ok(stream) }
    /// #
    /// # smol::block_on(tcp_connect()).unwrap();
    /// ```
    fn connect(addr: SocketAddr) -> Connect<Self>;

    /// Returns the socket address of the local half of this TCP connection.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # smol::block_on(async {
    /// #
    /// # use async_net_ as async_net;
    /// # use async_tcp::TcpListener;
    /// use async_tcp::TcpStream;
    /// use std::net::{IpAddr, SocketAddr};
    /// # use std::io::Result;
    /// use std::str::FromStr;
    ///
    /// # let addr = SocketAddr::from_str("127.0.0.1:0").unwrap();
    /// # let listener = async_net::TcpListener::bind(addr).await?;
    /// # let addr = listener.local_addr()?;
    /// // let addr = ...;
    /// #
    /// # let connect = smol::spawn(async move {
    /// let stream = async_net::TcpStream::connect(addr).await?;
    /// # Result::Ok(stream) });
    /// #
    /// # let (conn, peer_addr) = listener.accept().await?;
    /// # let stream = connect.await?;
    ///
    /// # fn tcp_local_addr<Stream: TcpStream>(stream: Stream) -> Result<()> {
    /// #
    /// assert_eq!(stream.local_addr()?.ip(), IpAddr::from_str("127.0.0.1").unwrap());
    /// #
    /// # Ok(()) }
    /// #
    /// # tcp_local_addr(stream)?;
    /// #
    /// # Result::<()>::Ok(()) }).unwrap();
    /// ```
    fn local_addr(&self) -> Result<SocketAddr>;

    /// Returns the socket address of the remote peer of this TCP connection.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # smol::block_on(async {
    /// #
    /// # use async_net_ as async_net;
    /// # use async_tcp::TcpListener;
    /// use async_tcp::TcpStream;
    /// use std::net::SocketAddr;
    /// # use std::io::Result;
    /// use std::str::FromStr;
    ///
    /// # let addr = SocketAddr::from_str("127.0.0.1:0").unwrap();
    /// # let listener = async_net::TcpListener::bind(addr).await?;
    /// # let addr = listener.local_addr()?;
    /// // let addr = ...;
    /// #
    /// # let connect = smol::spawn(async move {
    /// let stream = async_net::TcpStream::connect(addr).await?;
    /// # Result::Ok(stream) });
    /// #
    /// # let (conn, peer_addr) = listener.accept().await?;
    /// # let stream = connect.await?;
    ///
    /// # fn tcp_peer_addr<Stream: TcpStream>(addr: SocketAddr, stream: Stream) -> Result<()> {
    /// #
    /// assert_eq!(addr, stream.peer_addr()?);
    /// #
    /// # Ok(()) }
    /// #
    /// # tcp_peer_addr(addr, stream)?;
    /// #
    /// # Result::<()>::Ok(()) }).unwrap();
    /// ```
    fn peer_addr(&self) -> Result<SocketAddr>;

    /// Shuts down the read half, write half, or both halves of this connection.
    ///
    /// This method will cause all pending and future I/O in the given directions to return
    /// immediately with an appropriate value (see the documentation of [`Shutdown`]).
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # smol::block_on(async {
    /// #
    /// # use async_net_ as async_net;
    /// # use async_tcp::TcpListener;
    /// use async_tcp::TcpStream;
    /// use std::net::{Shutdown, SocketAddr};
    /// # use std::io::Result;
    /// use std::str::FromStr;
    ///
    /// # let addr = SocketAddr::from_str("127.0.0.1:0").unwrap();
    /// # let listener = async_net::TcpListener::bind(addr).await?;
    /// # let addr = listener.local_addr()?;
    /// // let addr = ...;
    /// #
    /// # let connect = smol::spawn(async move {
    /// let stream = async_net::TcpStream::connect(addr).await?;
    /// # Result::Ok(stream) });
    /// #
    /// # let (conn, peer_addr) = listener.accept().await?;
    /// # let stream = connect.await?;
    ///
    /// # fn tcp_shutdown<Stream: TcpStream>(stream: Stream) -> Result<()> {
    /// #
    /// stream.shutdown(Shutdown::Both)?;
    /// #
    /// # Ok(()) }
    /// #
    /// # tcp_shutdown(stream)?;
    /// #
    /// # Result::<()>::Ok(()) }).unwrap();
    /// ```
    fn shutdown(&self, how: Shutdown) -> Result<()>;

    /// Gets the value of the `TCP_NODELAY` option on this socket.
    ///
    /// For more information about this option, see [`set_nodelay()`].
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # smol::block_on(async {
    /// #
    /// # use async_net_ as async_net;
    /// # use async_tcp::TcpListener;
    /// use async_tcp::TcpStream;
    /// use std::net::SocketAddr;
    /// # use std::io::Result;
    /// use std::str::FromStr;
    ///
    /// # let addr = SocketAddr::from_str("127.0.0.1:0").unwrap();
    /// # let listener = async_net::TcpListener::bind(addr).await?;
    /// # let addr = listener.local_addr()?;
    /// // let addr = ...;
    /// #
    /// # let connect = smol::spawn(async move {
    /// let stream = async_net::TcpStream::connect(addr).await?;
    /// # Result::Ok(stream) });
    /// #
    /// # let (conn, peer_addr) = listener.accept().await?;
    /// # let stream = connect.await?;
    ///
    /// # fn tcp_nodelay<Stream: TcpStream>(stream: Stream) -> Result<()> {
    /// #
    /// stream.set_nodelay(true)?;
    /// assert_eq!(stream.nodelay()?, true);
    /// #
    /// # Ok(()) }
    /// #
    /// # tcp_nodelay(stream)?;
    /// #
    /// # Result::<()>::Ok(()) }).unwrap();
    /// ```
    ///
    /// [`set_nodelay()`]: TcpStream::set_nodelay()
    fn nodelay(&self) -> Result<bool>;

    /// Sets the value of `TCP_NODELAY` option on this socket.
    ///
    /// If set, this option disables the Nagle algorithm. This means that segments are always sent
    /// as soon as possible, even if there is only a small amount of data. When not set, data is
    /// buffered until there is a sufficient amount to send out, thereby avoiding the frequent
    /// reading of small packets.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # smol::block_on(async {
    /// #
    /// # use async_net_ as async_net;
    /// # use async_tcp::TcpListener;
    /// use async_tcp::TcpStream;
    /// use std::net::SocketAddr;
    /// # use std::io::Result;
    /// use std::str::FromStr;
    ///
    /// # let addr = SocketAddr::from_str("127.0.0.1:0").unwrap();
    /// # let listener = async_net::TcpListener::bind(addr).await?;
    /// # let addr = listener.local_addr()?;
    /// // let addr = ...;
    /// #
    /// # let connect = smol::spawn(async move {
    /// let stream = async_net::TcpStream::connect(addr).await?;
    /// # Result::Ok(stream) });
    /// #
    /// # let (conn, peer_addr) = listener.accept().await?;
    /// # let stream = connect.await?;
    ///
    /// # fn tcp_set_nodelay<Stream: TcpStream>(stream: Stream) -> Result<()> {
    /// #
    /// stream.set_nodelay(true)?;
    /// assert_eq!(stream.nodelay()?, true);
    /// #
    /// # Ok(()) }
    /// #
    /// # tcp_set_nodelay(stream)?;
    /// #
    /// # Result::<()>::Ok(()) }).unwrap();
    /// ```
    fn set_nodelay(&self, nodelay: bool) -> Result<()>;

    /// Gets the value of the `IP_TTL` option on this socket.
    ///
    /// For more information about this option, see [`set_ttl()`].
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # smol::block_on(async {
    /// #
    /// # use async_net_ as async_net;
    /// # use async_tcp::TcpListener;
    /// use async_tcp::TcpStream;
    /// use std::net::SocketAddr;
    /// # use std::io::Result;
    /// use std::str::FromStr;
    ///
    /// # let addr = SocketAddr::from_str("127.0.0.1:0").unwrap();
    /// # let listener = async_net::TcpListener::bind(addr).await?;
    /// # let addr = listener.local_addr()?;
    /// // let addr = ...;
    /// #
    /// # let connect = smol::spawn(async move {
    /// let stream = async_net::TcpStream::connect(addr).await?;
    /// # Result::Ok(stream) });
    /// #
    /// # let (conn, peer_addr) = listener.accept().await?;
    /// # let stream = connect.await?;
    ///
    /// # fn tcp_ttl<Stream: TcpStream>(stream: Stream) -> Result<()> {
    /// #
    /// stream.set_ttl(100)?;
    /// assert_eq!(stream.ttl()?, 100);
    /// #
    /// # Ok(()) }
    /// #
    /// # tcp_ttl(stream)?;
    /// #
    /// # Result::<()>::Ok(()) }).unwrap();
    /// ```
    ///
    /// [`set_ttl()`]: TcpStream::set_ttl()
    fn ttl(&self) -> Result<u32>;

    /// Sets the value for the `IP_TTL` option on this socket.
    ///
    /// This value sets the time-to-live field that is used in every packet sent from this socket.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # smol::block_on(async {
    /// #
    /// # use async_net_ as async_net;
    /// # use async_tcp::TcpListener;
    /// use async_tcp::TcpStream;
    /// use std::net::SocketAddr;
    /// # use std::io::Result;
    /// use std::str::FromStr;
    ///
    /// # let addr = SocketAddr::from_str("127.0.0.1:0").unwrap();
    /// # let listener = async_net::TcpListener::bind(addr).await?;
    /// # let addr = listener.local_addr()?;
    /// // let addr = ...;
    /// #
    /// # let connect = smol::spawn(async move {
    /// let stream = async_net::TcpStream::connect(addr).await?;
    /// # Result::Ok(stream) });
    /// #
    /// # let (conn, peer_addr) = listener.accept().await?;
    /// # let stream = connect.await?;
    ///
    /// # fn tcp_set_ttl<Stream: TcpStream>(stream: Stream) -> Result<()> {
    /// #
    /// stream.set_ttl(100)?;
    /// assert_eq!(stream.ttl()?, 100);
    /// #
    /// # Ok(()) }
    /// #
    /// # tcp_set_ttl(stream)?;
    /// #
    /// # Result::<()>::Ok(()) }).unwrap();
    /// ```
    fn set_ttl(&self, ttl: u32) -> Result<()>;
}

// ============================================ Types =========================================== \\

/// Future returned by the [`TcpListener::bind()`] method.
///
/// If you are implementing [`TcpListener`] manually, you can construct a new instance of `Bind`
/// using its implementation of `From<Pin<Box<dyn Future<Output = Result<Listener>> + Send>>>`.
pub struct Bind<Listener> {
    fut: Pin<Box<dyn Future<Output = Result<Listener>> + Send>>,
}

/// Future returned by the [`TcpListener::accept()`] method.
pub struct Accept<'listener, Listener> {
    listener: &'listener Listener,
}

/// Stream returned by the [`TcpListener::incoming()`] method.
pub struct Incoming<'listener, Listener> {
    listener: &'listener Listener,
}

/// Future returned by the [`TcpStream::connect()`] method.
///
/// If you are implementing [`TcpStream`] manually, you can construct a new instance of `Connect`
/// using its implementation of `From<Pin<Box<dyn Future<Output = Result<Stream>> + Send>>>`.
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
