use crate::{configuration::Configuration, rpc_apis::ApiSet};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq)]
pub struct RpcEndpoint {
    pub interface: String,
    pub port: u16,
    pub apis: ApiSet,
}

impl FromStr for RpcEndpoint {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let split: Vec<&str> = s.split('|').collect();
        if split.len() != 2 {
            return Err(format!("Invalid additional endpoint given. Expected format: [host:port|api0;api1], but got: {}", s));
        }

        let url: Vec<&str> = split[0].split(':').collect();
        if url.len() != 2 || url[0].len() == 0 {
            return Err(format!("Invalid additional endpoint given. Expected format: [host:port|api0;api1], but got: {}", s));
        }

        Ok(Self {
            interface: Configuration::map_interface(url[0].into()),
            port: url[1].parse::<u16>().map_err(|e| e.to_string())?,
            apis: split[1].replace(";", ",").parse()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::RpcEndpoint;
    use crate::rpc_apis::{
        Api::{self, *},
        ApiSet,
    };

    fn make_apis(apis: Vec<Api>) -> ApiSet {
        ApiSet::List(apis.into_iter().collect())
    }

    #[test]
    fn should_parse_successfully() {
        assert_eq!(
            "127.0.0.1:9999|web3".parse::<RpcEndpoint>(),
            Ok(RpcEndpoint {
                interface: "127.0.0.1".into(),
                port: 9999,
                apis: make_apis(vec![Api::Web3]),
            })
        );
        assert_eq!(
            "local:80|web3;eth;net".parse::<RpcEndpoint>(),
            Ok(RpcEndpoint {
                interface: "127.0.0.1".into(),
                port: 80,
                apis: make_apis(vec![Web3, Eth, Net]),
            })
        );
        assert_eq!(
            "all:13789|all".parse::<RpcEndpoint>(),
            Ok(RpcEndpoint {
                interface: "0.0.0.0".into(),
                port: 13789,
                apis: ApiSet::All,
            })
        );
        assert_eq!(
            "192.168.0.1:1|all".parse::<RpcEndpoint>(),
            Ok(RpcEndpoint {
                interface: "192.168.0.1".into(),
                port: 1,
                apis: make_apis(vec![
                    Eth,
                    ParitySet,
                    Net,
                    Debug,
                    Traces,
                    Personal,
                    Parity,
                    ParityAccounts,
                    Web3,
                    Rpc,
                    ParityPubSub,
                    EthPubSub,
                    SecretStore,
                    Signer
                ]),
            })
        );
    }

    #[test]
    fn should_parse_with_error() {
        assert!("127.0.0.1:9999|".parse::<RpcEndpoint>().is_err());
        assert!("127.0.0.1:|web3".parse::<RpcEndpoint>().is_err());
        assert!(":9999|web3".parse::<RpcEndpoint>().is_err());
        assert!("local:9999;web3;net".parse::<RpcEndpoint>().is_err());
        assert!("local:9999|web3;net;".parse::<RpcEndpoint>().is_err());
        assert!("local;9999|web3;net".parse::<RpcEndpoint>().is_err());
        assert!("local:9999|web3|net".parse::<RpcEndpoint>().is_err());
        assert!("local|9999:web3".parse::<RpcEndpoint>().is_err());
        assert!("local9999|web3".parse::<RpcEndpoint>().is_err());
    }
}
