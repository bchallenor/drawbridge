mod dispatch;
mod parse;

pub use cli::dispatch::dispatch;
pub use cli::parse::parse_from_safe;

use cloud::InstanceType;
use ipnet::IpNet;
use iprules::IpProtocol;

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
