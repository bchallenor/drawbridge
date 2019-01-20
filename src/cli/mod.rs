mod dispatch;
mod parse;

pub use crate::cli::dispatch::dispatch;
pub use crate::cli::parse::parse_from_safe;

use crate::cloud::InstanceType;
use crate::iprules::IpProtocol;
use ipnet::IpNet;

#[derive(Debug, Eq, PartialEq)]
pub enum Command {
    Open {
        ip_cidrs: Vec<IpNet>,
        ip_protocols: Vec<IpProtocol>,
        names: Vec<String>,
    },
    Close {
        names: Vec<String>,
    },
    Start {
        instance_type: Option<InstanceType>,
        names: Vec<String>,
    },
    Stop {
        names: Vec<String>,
    },
}
