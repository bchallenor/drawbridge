pub mod aws;
pub mod mem;

use failure::Error;
use iprules::IpIngressRule;
use std::collections::HashSet;
use std::fmt;
use std::net::Ipv4Addr;
use std::str;

pub trait Cloud {
    type Firewall: Firewall;
    type Instance: Instance;

    fn list_firewalls(&self) -> Result<Vec<Self::Firewall>, Error>;
    fn list_instances(&self) -> Result<Vec<Self::Instance>, Error>;
}

pub trait Firewall: fmt::Debug {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn list_ingress_rules(&self) -> Result<HashSet<IpIngressRule>, Error>;
    fn add_ingress_rules<'a, R>(&self, rules: R) -> Result<(), Error>
    where
        R: IntoIterator<Item = &'a IpIngressRule>;
    fn remove_ingress_rules<'a, R>(&self, rules: R) -> Result<(), Error>
    where
        R: IntoIterator<Item = &'a IpIngressRule>;
}

pub trait Instance: fmt::Debug {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn fqdn(&self) -> Option<&str>;
    // requires the instance to be stopped
    fn try_ensure_instance_type(&self, instance_type: &InstanceType) -> Result<(), Error>;
    fn ensure_running(&self) -> Result<InstanceRunningState, Error>;
    fn ensure_stopped(&self) -> Result<(), Error>;
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct InstanceType(String);

impl InstanceType {
    pub fn new<S: Into<String>>(s: S) -> InstanceType {
        InstanceType(s.into())
    }
}

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
