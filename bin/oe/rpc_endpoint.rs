use crate::{configuration::Configuration, rpc_apis::ApiSet};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq)]
pub struct Endpoint {
    pub interface: String,
    pub port: u16,
    pub apis: ApiSet,
}

impl FromStr for Endpoint {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let split: Vec<&str> = s.split('|').collect();
        if split.len() != 2 {
            return Err(format!("Invalid additional endpoint given. Expected format: [host:port|api0;api1], but got: {}", s));
        }

        let url: Vec<&str> = split[0].split(':').collect();
        if url.len() != 2 {
            return Err(format!("Invalid additional endpoint given. Expected format: [host:port|api0;api1], but got: {}", s));
        }

        Ok(Self {
            interface: Configuration::map_interface(url[0].into()),
            port: url[1].parse::<u16>().map_err(|e| e.to_string())?,
            apis: split[1].replace(";", ",").parse()?,
        })
    }
}
