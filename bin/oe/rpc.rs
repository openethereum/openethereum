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

use std::{collections::HashSet, io, path::PathBuf, sync::Arc};

use crate::{
    helpers::parity_ipc_path,
    rpc_apis::{self, ApiSet},
    rpc_endpoint::RpcEndpoint,
};
use dir::{default_data_path, helpers::replace_home};
use jsonrpc_core::MetaIoHandler;
use parity_rpc::{
    self as rpc,
    informant::{Middleware, RpcStats},
    DomainsValidation, Metadata,
};
use parity_runtime::Executor;

pub use parity_rpc::{HttpServer, IpcServer, RequestMiddleware};
//pub use parity_rpc::ws::Server as WsServer;
pub use parity_rpc::ws::{ws, Server as WsServer};

pub const DAPPS_DOMAIN: &'static str = "web3.site";

#[derive(Debug, Clone, PartialEq)]
pub struct HttpConfiguration {
    pub enabled: bool,
    pub interface: String,
    pub port: u16,
    pub apis: ApiSet,
    pub cors: Option<Vec<String>>,
    pub hosts: Option<Vec<String>>,
    pub additional_endpoints: Vec<RpcEndpoint>,
    pub server_threads: usize,
    pub processing_threads: usize,
    pub max_payload: usize,
    pub keep_alive: bool,
}

impl Default for HttpConfiguration {
    fn default() -> Self {
        HttpConfiguration {
            enabled: true,
            interface: "127.0.0.1".into(),
            port: 8545,
            apis: ApiSet::UnsafeContext,
            cors: Some(vec![]),
            hosts: Some(vec![]),
            additional_endpoints: vec![],
            server_threads: 1,
            processing_threads: 4,
            max_payload: 5,
            keep_alive: true,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct IpcConfiguration {
    pub enabled: bool,
    pub socket_addr: String,
    pub apis: ApiSet,
}

impl Default for IpcConfiguration {
    fn default() -> Self {
        IpcConfiguration {
            enabled: true,
            socket_addr: if cfg!(windows) {
                r"\\.\pipe\jsonrpc.ipc".into()
            } else {
                let data_dir = ::dir::default_data_path();
                parity_ipc_path(&data_dir, "$BASE/jsonrpc.ipc", 0)
            },
            apis: ApiSet::IpcContext,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WsConfiguration {
    pub enabled: bool,
    pub interface: String,
    pub port: u16,
    pub apis: ApiSet,
    pub additional_endpoints: Vec<RpcEndpoint>,
    pub max_connections: usize,
    pub origins: Option<Vec<String>>,
    pub hosts: Option<Vec<String>>,
    pub signer_path: PathBuf,
    pub support_token_api: bool,
    pub max_payload: usize,
}

impl Default for WsConfiguration {
    fn default() -> Self {
        let data_dir = default_data_path();
        WsConfiguration {
            enabled: true,
            interface: "127.0.0.1".into(),
            port: 8546,
            apis: ApiSet::UnsafeContext,
            additional_endpoints: vec![],
            max_connections: 100,
            origins: Some(vec![
                "parity://*".into(),
                "chrome-extension://*".into(),
                "moz-extension://*".into(),
            ]),
            hosts: Some(Vec::new()),
            signer_path: replace_home(&data_dir, "$BASE/signer").into(),
            support_token_api: true,
            max_payload: 5,
        }
    }
}

impl WsConfiguration {
    pub fn address(&self) -> Option<rpc::Host> {
        address(self.enabled, &self.interface, self.port, &self.hosts)
    }
}

fn address(
    enabled: bool,
    bind_iface: &str,
    bind_port: u16,
    hosts: &Option<Vec<String>>,
) -> Option<rpc::Host> {
    if !enabled {
        return None;
    }

    match *hosts {
        Some(ref hosts) if !hosts.is_empty() => Some(hosts[0].clone().into()),
        _ => Some(format!("{}:{}", bind_iface, bind_port).into()),
    }
}

pub struct Dependencies<D: rpc_apis::Dependencies> {
    pub apis: Arc<D>,
    pub executor: Executor,
    pub stats: Arc<RpcStats>,
}

pub fn new_ws<D: rpc_apis::Dependencies>(
    conf: WsConfiguration,
    deps: &Dependencies<D>,
) -> Result<Vec<WsServer>, String> {
    if !conf.enabled {
        return Ok(vec![]);
    }

    let domain = DAPPS_DOMAIN;
    let allowed_origins = into_domains(with_domain(conf.origins, domain, &None));

    let signer_path;
    let path = match conf.support_token_api {
        true => {
            signer_path = crate::signer::codes_path(&conf.signer_path);
            Some(signer_path.as_path())
        }
        false => None,
    };

    let mut endpoints = conf.additional_endpoints;
    endpoints.push(RpcEndpoint {
        interface: conf.interface,
        port: conf.port,
        apis: conf.apis,
    });

    let max_connections = conf.max_connections;
    let max_payload = conf.max_payload;
    let hosts = conf.hosts;

    endpoints
        .into_iter()
        .map(|endpoint| {
            let url = format!("{}:{}", endpoint.interface, endpoint.port);
            let addr = url
                .parse()
                .map_err(|_| format!("Invalid WebSockets listen host/port given: {}", url))?;

            let full_handler = setup_apis(rpc_apis::ApiSet::All, deps);
            let handler = {
                let mut handler = MetaIoHandler::with_middleware((
                    rpc::WsDispatcher::new(full_handler),
                    Middleware::new(deps.stats.clone(), deps.apis.activity_notifier()),
                ));
                let apis = endpoint.apis.list_apis();
                deps.apis.extend_with_set(&mut handler, &apis);

                handler
            };
            let allowed_hosts = into_domains(with_domain(hosts.clone(), domain, &Some(url.clone().into())));

            rpc_servers::start_ws(
                &addr,
                handler,
                allowed_origins.clone(),
                allowed_hosts,
                max_connections,
                rpc::WsExtractor::new(path.clone()),
                rpc::WsExtractor::new(path.clone()),
                rpc::WsStats::new(deps.stats.clone()),
                max_payload,
            )
            .map_err(|e| {
                match e {
                    rpc::ws::Error::WsError(ws::Error {
                        kind: ws::ErrorKind::Io(ref err), ..
                    }) if err.kind() == io::ErrorKind::AddrInUse =>
                        format!("WebSockets address {} is already in use, make sure that another instance of an Ethereum client is not running or change the address using the --ws-port and --ws-interface options.", url),
                    _ => format!("WebSockets error: {:?}", e),
                }
            })
        })
        .collect()
}

pub fn new_http<D: rpc_apis::Dependencies>(
    id: &str,
    options: &str,
    conf: HttpConfiguration,
    deps: &Dependencies<D>,
) -> Result<Vec<HttpServer>, String> {
    if !conf.enabled {
        return Ok(vec![]);
    }

    let domain = DAPPS_DOMAIN;

    let cors_domains = into_domains(conf.cors);
    let health_api = Some(("/api/health", "parity_nodeStatus"));

    let mut endpoints = conf.additional_endpoints;
    endpoints.push(RpcEndpoint {
        interface: conf.interface,
        port: conf.port,
        apis: conf.apis,
    });

    let hosts = conf.hosts;
    let server_threads = conf.server_threads;
    let max_payload = conf.max_payload;
    let keep_alive = conf.keep_alive;

    endpoints.into_iter().map(|endpoint| {
        let url = format!("{}:{}", endpoint.interface, endpoint.port);
        let addr = url
            .parse()
            .map_err(|_| format!("Invalid {} listen host/port given: {}", id, url))?;
        let handler = setup_apis(endpoint.apis, deps);
        let allowed_hosts = into_domains(with_domain(hosts.clone(), domain, &Some(url.clone().into())));

        rpc_servers::start_http(
            &addr,
            cors_domains.clone(),
            allowed_hosts,
            health_api,
            handler,
            rpc::RpcExtractor,
            server_threads,
            max_payload,
            keep_alive,
        ).map_err(|e| {
            if e.kind() == io::ErrorKind::AddrInUse {
                format!("{} address {} is already in use, make sure that another instance of an Ethereum client is not running or change the address using the --{}-port and --{}-interface options.", id, url, options, options)
            } else {
                format!("{} error: {:?}", id, e)
            }
        })
    }).collect()
}

pub fn new_ipc<D: rpc_apis::Dependencies>(
    conf: IpcConfiguration,
    dependencies: &Dependencies<D>,
) -> Result<Option<IpcServer>, String> {
    if !conf.enabled {
        return Ok(None);
    }

    let handler = setup_apis(conf.apis, dependencies);
    let path = PathBuf::from(&conf.socket_addr);
    // Make sure socket file can be created on unix-like OS.
    // Windows pipe paths are not on the FS.
    if !cfg!(windows) {
        if let Some(dir) = path.parent() {
            ::std::fs::create_dir_all(&dir).map_err(|err| {
                format!(
                    "Unable to create IPC directory at {}: {}",
                    dir.display(),
                    err
                )
            })?;
        }
    }

    match rpc_servers::start_ipc(&conf.socket_addr, handler, rpc::RpcExtractor) {
        Ok(server) => Ok(Some(server)),
        Err(io_error) => Err(format!("IPC error: {}", io_error)),
    }
}

fn into_domains<T: From<String>>(items: Option<Vec<String>>) -> DomainsValidation<T> {
    items
        .map(|vals| vals.into_iter().map(T::from).collect())
        .into()
}

fn with_domain(
    items: Option<Vec<String>>,
    domain: &str,
    dapps_address: &Option<rpc::Host>,
) -> Option<Vec<String>> {
    fn extract_port(s: &str) -> Option<u16> {
        s.split(':').nth(1).and_then(|s| s.parse().ok())
    }

    items.map(move |items| {
        let mut items = items.into_iter().collect::<HashSet<_>>();
        {
            let mut add_hosts = |address: &Option<rpc::Host>| {
                if let Some(host) = address.clone() {
                    items.insert(host.to_string());
                    items.insert(host.replace("127.0.0.1", "localhost"));
                    items.insert(format!("http://*.{}", domain)); //proxypac
                    if let Some(port) = extract_port(&*host) {
                        items.insert(format!("http://*.{}:{}", domain, port));
                    }
                }
            };

            add_hosts(dapps_address);
        }
        items.into_iter().collect()
    })
}

pub fn setup_apis<D>(
    apis: ApiSet,
    deps: &Dependencies<D>,
) -> MetaIoHandler<Metadata, Middleware<D::Notifier>>
where
    D: rpc_apis::Dependencies,
{
    let mut handler = MetaIoHandler::with_middleware(Middleware::new(
        deps.stats.clone(),
        deps.apis.activity_notifier(),
    ));
    let apis = apis.list_apis();
    deps.apis.extend_with_set(&mut handler, &apis);

    handler
}

#[cfg(test)]
mod tests {
    use super::address;

    #[test]
    fn should_return_proper_address() {
        assert_eq!(address(false, "localhost", 8180, &None), None);
        assert_eq!(
            address(true, "localhost", 8180, &None),
            Some("localhost:8180".into())
        );
        assert_eq!(
            address(true, "localhost", 8180, &Some(vec!["host:443".into()])),
            Some("host:443".into())
        );
        assert_eq!(
            address(true, "localhost", 8180, &Some(vec!["host".into()])),
            Some("host".into())
        );
    }
}
