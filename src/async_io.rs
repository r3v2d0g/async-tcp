/**************************************************************************************************
 *                                                                                                *
 * This Source Code Form is subject to the terms of the Mozilla Public                            *
 * License, v. 2.0. If a copy of the MPL was not distributed with this                            *
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.                                       *
 *                                                                                                *
 **************************************************************************************************/

// =========================================== Imports ========================================== \\

use crate::{Bind, Connect, TcpListener, TcpStream};
use async_io_::Async;
use core::future::Future;
use core::task::{Context, Poll};
use futures_lite::{future, pin, FutureExt};
use std::io::Result;
use std::net::{self, Shutdown, SocketAddr};

// ====================================== impl TcpListener ====================================== \\

impl TcpListener for Async<net::TcpListener> {
    type Stream = Async<net::TcpStream>;

    fn bind(addr: SocketAddr) -> Bind<Self> {
        Bind::from(future::ready(Self::bind(addr)).boxed())
    }

    fn local_addr(&self) -> Result<SocketAddr> {
        self.get_ref().local_addr()
    }

    fn poll_accept(&self, ctx: &mut Context) -> Poll<Result<(Self::Stream, SocketAddr)>> {
        let fut = Self::accept(self);
        pin!(fut);

        fut.poll(ctx)
    }

    fn ttl(&self) -> Result<u32> {
        self.get_ref().ttl()
    }

    fn set_ttl(&self, ttl: u32) -> Result<()> {
        self.get_ref().set_ttl(ttl)
    }
}

// ======================================= impl TcpStream ======================================= \\

impl TcpStream for Async<net::TcpStream> {
    fn connect(addr: SocketAddr) -> Connect<Self> {
        Connect::from(Self::connect(addr).boxed())
    }

    fn local_addr(&self) -> Result<SocketAddr> {
        self.get_ref().local_addr()
    }

    fn peer_addr(&self) -> Result<SocketAddr> {
        self.get_ref().peer_addr()
    }

    fn shutdown(&self, how: Shutdown) -> Result<()> {
        self.get_ref().shutdown(how)
    }

    fn nodelay(&self) -> Result<bool> {
        self.get_ref().nodelay()
    }

    fn set_nodelay(&self, nodelay: bool) -> Result<()> {
        self.get_ref().set_nodelay(nodelay)
    }

    fn ttl(&self) -> Result<u32> {
        self.get_ref().ttl()
    }

    fn set_ttl(&self, ttl: u32) -> Result<()> {
        self.get_ref().set_ttl(ttl)
    }
}
