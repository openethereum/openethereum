use rpc_server::http;
use std::collections::HashSet;

pub(crate) const DAPPS_DOMAIN: &'static str = "web3.site";

pub(crate) fn into_domains<T: From<String>>(
    items: Option<Vec<String>>,
) -> http::DomainsValidation<T> {
    items
        .map(|vals| vals.into_iter().map(T::from).collect())
        .into()
}

pub(crate) fn with_domain(
    items: Option<Vec<String>>,
    domain: &str,
    dapps_address: &Option<http::Host>,
) -> Option<Vec<String>> {
    fn extract_port(s: &str) -> Option<u16> {
        s.split(':').nth(1).and_then(|s| s.parse().ok())
    }

    items.map(move |items| {
        let mut items = items.into_iter().collect::<HashSet<_>>();
        {
            let mut add_hosts = |address: &Option<http::Host>| {
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
