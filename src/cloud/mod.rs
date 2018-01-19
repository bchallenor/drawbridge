pub mod aws;
pub mod mem;

use errors::*;
use iprules::IpIngressRule;
use std::collections::HashSet;
use std::fmt;
use std::net::Ipv4Addr;
use std::str;

pub trait Cloud {
    type Firewall: Firewall;
    type Instance: Instance;

    fn list_firewalls(&self) -> Result<Vec<Self::Firewall>>;
    fn list_instances(&self) -> Result<Vec<Self::Instance>>;
}

pub trait Firewall: fmt::Debug {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn list_ingress_rules(&self) -> Result<HashSet<IpIngressRule>>;
    fn add_ingress_rules<'a, R>(&self, rules: R) -> Result<()>
    where
        R: IntoIterator<Item = &'a IpIngressRule>;
    fn remove_ingress_rules<'a, R>(&self, rules: R) -> Result<()>
    where
        R: IntoIterator<Item = &'a IpIngressRule>;
}

pub trait Instance: fmt::Debug {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn fqdn(&self) -> Option<&str>;
    fn ensure_running(&self, instance_type: &InstanceType) -> Result<InstanceRunningState>;
    fn ensure_stopped(&self) -> Result<()>;
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct InstanceType(pub String);

impl fmt::Display for InstanceType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for InstanceType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceRunningState {
    pub instance_type: InstanceType,
    pub ip_addr: Ipv4Addr,
}
