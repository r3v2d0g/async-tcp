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

pub trait TcpListener: Sized {
    type Stream: TcpStream;

    fn bind(addr: SocketAddr) -> Bind<Self>;

    fn local_addr(&self) -> Result<SocketAddr>;

    fn accept(&self) -> Accept<Self> {
        Accept { listener: self }
    }

    fn poll_accept(&self, ctx: &mut Context) -> Poll<Result<(Self::Stream, SocketAddr)>>;

    fn incoming(&self) -> Incoming<Self> {
        Incoming { listener: self }
    }

    fn ttl(&self) -> Result<u32>;

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
