use std::io;

use engine_api::v1::{Engine, EngineClient};
use parity_rpc::hyper::{Body, Request};
use rpc_server::{http, HttpServer, IoHandler};

use engine_api_apis::EthClientDependencies;
use rpc_utils::{into_domains, with_domain, DAPPS_DOMAIN};

#[derive(Debug, Clone, PartialEq)]
pub struct HttpConfiguration {
    pub enabled: bool,
    pub interface: String,
    pub port: u16,
    pub cors: Option<Vec<String>>,
    pub hosts: Option<Vec<String>>,
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
            port: 8550,
            cors: Some(vec![]),
            hosts: Some(vec![]),
            server_threads: 1,
            processing_threads: 4,
            max_payload: 5,
            keep_alive: true,
        }
    }
}

struct HttpExtractor;

impl http::MetaExtractor<()> for HttpExtractor {
    fn read_metadata(&self, _: &Request<Body>) -> () {
        ()
    }
}

pub fn new_http(
    id: &str,
    options: &str,
    conf: HttpConfiguration,
    engine_client: EngineClient,
    eth_deps: EthClientDependencies,
) -> Result<Option<HttpServer>, String> {
    if !conf.enabled {
        return Ok(None);
    }

    let domain = DAPPS_DOMAIN;
    let url = format!("{}:{}", conf.interface, conf.port);
    let addr = url
        .parse()
        .map_err(|_| format!("Invalid {} listen host/port given: {}", id, url))?;
    let handler = setup_apis(engine_client, eth_deps);

    let cors_domains = into_domains(conf.cors);
    let allowed_hosts = into_domains(with_domain(conf.hosts, domain, &Some(url.clone().into())));

    let extractor = HttpExtractor {};

    let start_result = rpc_server::start_http(
        &addr,
        cors_domains,
        allowed_hosts,
        None::<(String, String)>,
        handler,
        extractor,
        conf.server_threads,
        conf.max_payload,
        conf.keep_alive,
    );

    match start_result {
        Ok(server) => Ok(Some(server)),
        Err(ref err) if err.kind() == io::ErrorKind::AddrInUse => Err(
            format!("{} address {} is already in use, make sure that another instance of an Ethereum client is not running or change the address using the --{}-port and --{}-interface options.", id, url, options, options)
        ),
        Err(e) => Err(format!("{} error: {:?}", id, e)),
    }
}

fn setup_apis(engine: EngineClient, eth_deps: EthClientDependencies) -> IoHandler {
    let mut handler = IoHandler::new();
    handler.extend_with(engine.to_delegate());
    eth_deps.extend_api(&mut handler);

    handler
}
