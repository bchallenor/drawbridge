mod parse;
mod dispatch;

pub use cli::dispatch::dispatch;
pub use cli::parse::parse;

use cloud::InstanceType;
use ipnet::Ipv4Net;
use iprules::IpProtocol;

#[derive(Debug)]
pub enum Command {
    Start {
        ip_cidrs: Vec<Ipv4Net>,
        ip_protocols: Vec<IpProtocol>,
        instance_type: Option<InstanceType>,
    },
    Stop,
}
