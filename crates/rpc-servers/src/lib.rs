// Copyright 2015-2020 Parity Technologies (UK) Ltd.
// This file is part of OpenEthereum.

// OpenEthereum is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// OpenEthereum is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with OpenEthereum.  If not, see <http://www.gnu.org/licenses/>.

//! OpenEthereum RPC Servers (WS, HTTP, IPC).

#![warn(missing_docs)]

use std::{io, net::SocketAddr};

pub use jsonrpc_core::{MetaIoHandler, Metadata, Middleware};

/// Type alias for ipc server
pub type IpcServer = ipc::Server;
/// Type alias for http server
pub type HttpServer = http::Server;
/// Type alias for ws server
pub type WsServer = ws::Server;

/// Start http server asynchronously and returns result with `Server` handle on success or an error.
pub fn start_http<M, S, H, T, A, B>(
    addr: &SocketAddr,
    cors_domains: http::DomainsValidation<http::AccessControlAllowOrigin>,
    allowed_hosts: http::DomainsValidation<http::Host>,
    health_api: Option<(A, B)>,
    handler: H,
    extractor: T,
    threads: usize,
    max_payload: usize,
    keep_alive: bool,
) -> ::std::io::Result<HttpServer>
where
    M: Metadata + Unpin,
    S: Middleware<M>,
    S::Future: Unpin,
    S::CallFuture: Unpin,
    H: Into<MetaIoHandler<M, S>>,
    T: http::MetaExtractor<M>,
    A: Into<String>,
    B: Into<String>,
{
    Ok(http::ServerBuilder::with_meta_extractor(handler, extractor)
        .keep_alive(keep_alive)
        .threads(threads)
        .cors(cors_domains)
        .allowed_hosts(allowed_hosts)
        .health_api(health_api)
        .cors_allow_headers(http::cors::AccessControlAllowHeaders::Any)
        .max_request_body_size(max_payload * 1024 * 1024)
        .start_http(addr)?)
}

/// Same as `start_http`, but takes an additional `middleware` parameter that is introduced as a
/// hyper middleware.
pub fn start_http_with_middleware<M, S, H, T, R>(
    addr: &SocketAddr,
    cors_domains: http::DomainsValidation<http::AccessControlAllowOrigin>,
    allowed_hosts: http::DomainsValidation<http::Host>,
    handler: H,
    extractor: T,
    middleware: R,
    threads: usize,
    max_payload: usize,
    keep_alive: bool,
) -> ::std::io::Result<HttpServer>
where
    M: Metadata + Unpin,
    S: Middleware<M>,
    S::Future: Unpin,
    S::CallFuture: Unpin,
    H: Into<jsonrpc_core::MetaIoHandler<M, S>>,
    T: http::MetaExtractor<M>,
    R: http::RequestMiddleware,
{
    Ok(http::ServerBuilder::with_meta_extractor(handler, extractor)
        .keep_alive(keep_alive)
        .threads(threads)
        .cors(cors_domains)
        .allowed_hosts(allowed_hosts)
        .cors_allow_headers(http::cors::AccessControlAllowHeaders::Any)
        .max_request_body_size(max_payload * 1024 * 1024)
        .request_middleware(middleware)
        .start_http(addr)?)
}

/// Start IPC server listening on given path.
pub fn start_ipc<M, S, H, T>(addr: &str, handler: H, extractor: T) -> io::Result<ipc::Server>
where
    M: jsonrpc_core::Metadata,
    S: jsonrpc_core::Middleware<M>,
    S::Future: Unpin,
    S::CallFuture: Unpin,
    H: Into<jsonrpc_core::MetaIoHandler<M, S>>,
    T: ipc::MetaExtractor<M>,
{
    ipc::ServerBuilder::with_meta_extractor(handler, extractor).start(addr)
}

/// Start WS server and return `Server` handle.
pub fn start_ws<M, S, H, T, U, V>(
    addr: &SocketAddr,
    handler: H,
    allowed_origins: ws::DomainsValidation<ws::Origin>,
    allowed_hosts: ws::DomainsValidation<ws::Host>,
    max_connections: usize,
    extractor: T,
    middleware: V,
    stats: U,
    max_payload: usize,
) -> Result<ws::Server, ws::Error>
where
    M: jsonrpc_core::Metadata + Unpin,
    S: jsonrpc_core::Middleware<M>,
    S::Future: Unpin,
    S::CallFuture: Unpin,
    H: Into<jsonrpc_core::MetaIoHandler<M, S>>,
    T: ws::MetaExtractor<M>,
    U: ws::SessionStats,
    V: ws::RequestMiddleware,
{
    ws::ServerBuilder::with_meta_extractor(handler, extractor)
        .request_middleware(middleware)
        .allowed_origins(allowed_origins)
        .allowed_hosts(allowed_hosts)
        .max_connections(max_connections)
        .max_payload(max_payload * 1024 * 1024)
        .session_stats(stats)
        .start(addr)
}
