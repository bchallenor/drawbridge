mod parse;
mod dispatch;

pub use cli::dispatch::dispatch;
pub use cli::parse::parse_from_safe;

use cloud::InstanceType;
use ipnet::Ipv4Net;
use iprules::IpProtocol;

#[derive(Debug, Eq, PartialEq)]
pub enum Command {
    Open {
        ip_cidrs: Vec<Ipv4Net>,
        ip_protocols: Vec<IpProtocol>,
        instance_type: Option<InstanceType>,
    },
    Close,
}
