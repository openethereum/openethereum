// Copyright 2015-2019 Parity Technologies (UK) Ltd.
// This file is part of Parity Ethereum.

// Parity Ethereum is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity Ethereum is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity Ethereum.  If not, see <http://www.gnu.org/licenses/>.

//! WebSockets server tests.

use std::sync::Arc;

use jsonrpc_core::MetaIoHandler;
use ws;

use tests::{helpers::Server, http_client};
use v1::{extractors, informant};

/// Setup a mock signer for tests
pub fn serve() -> (Server<ws::Server>, usize) {
    let address = "127.0.0.1:0".parse().unwrap();
    let io = MetaIoHandler::default();
    let stats = Arc::new(informant::RpcStats::default());

    let res = Server::new(|_| {
        ::start_ws(
            &address,
            io,
            ws::DomainsValidation::Disabled,
            ws::DomainsValidation::Disabled,
            5,
            extractors::WsExtractor::new(),
            extractors::WsExtractor::new(),
            extractors::WsStats::new(stats),
        )
        .unwrap()
    });
    let port = res.addr().port() as usize;

    (res, port)
}

/// Test a single request to running server
pub fn request(server: Server<ws::Server>, request: &str) -> http_client::Response {
    http_client::request(server.server.addr(), request)
}

#[cfg(test)]
mod testing {
    use super::{request, serve};
    #[test]
    fn should_not_redirect_to_parity_host() {
        // given
        let (server, port) = serve();

        // when
        let response = request(
            server,
            &format!(
                "\
				GET / HTTP/1.1\r\n\
				Host: 127.0.0.1:{}\r\n\
				Connection: close\r\n\
				\r\n\
				{{}}
			",
                port
            ),
        );

        // then
        assert_eq!(response.status, "HTTP/1.1 200 OK".to_owned());
    }
}
