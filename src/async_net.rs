/**************************************************************************************************
 *                                                                                                *
 * This Source Code Form is subject to the terms of the Mozilla Public                            *
 * License, v. 2.0. If a copy of the MPL was not distributed with this                            *
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.                                       *
 *                                                                                                *
 **************************************************************************************************/

// =========================================== Imports ========================================== \\

use crate::{Bind, Connect, TcpListener, TcpStream};
use async_net_ as net;
use core::future::Future;
use core::task::{Context, Poll};
use futures_lite::{pin, FutureExt};
use std::io::Result;
use std::net::{Shutdown, SocketAddr};

// ====================================== impl TcpListener ====================================== \\

impl TcpListener for net::TcpListener {
    type Stream = net::TcpStream;

    fn bind(addr: SocketAddr) -> Bind<Self> {
        Bind::from(Self::bind(addr).boxed())
    }

    fn local_addr(&self) -> Result<SocketAddr> {
        Self::local_addr(self)
    }

    fn poll_accept(&self, ctx: &mut Context) -> Poll<Result<(Self::Stream, SocketAddr)>> {
        let fut = Self::accept(self);
        pin!(fut);

        fut.poll(ctx)
    }

    fn ttl(&self) -> Result<u32> {
        Self::ttl(self)
    }

    fn set_ttl(&self, ttl: u32) -> Result<()> {
        Self::set_ttl(self, ttl)
    }
}

// ======================================= impl TcpStream ======================================= \\

impl TcpStream for net::TcpStream {
    fn connect(addr: SocketAddr) -> Connect<Self> {
        Connect::from(Self::connect(addr).boxed())
    }

    fn local_addr(&self) -> Result<SocketAddr> {
        Self::local_addr(self)
    }

    fn peer_addr(&self) -> Result<SocketAddr> {
        Self::peer_addr(self)
    }

    fn shutdown(&self, how: Shutdown) -> Result<()> {
        Self::shutdown(self, how)
    }

    fn nodelay(&self) -> Result<bool> {
        Self::nodelay(self)
    }

    fn set_nodelay(&self, nodelay: bool) -> Result<()> {
        Self::set_nodelay(self, nodelay)
    }

    fn ttl(&self) -> Result<u32> {
        Self::ttl(self)
    }

    fn set_ttl(&self, ttl: u32) -> Result<()> {
        Self::set_ttl(self, ttl)
    }
}
