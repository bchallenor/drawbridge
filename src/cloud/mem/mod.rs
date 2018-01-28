mod firewall;
mod instance;

use cloud::Cloud;
use cloud::Firewall;
use cloud::Instance;
use cloud::InstanceType;
pub use cloud::mem::firewall::MemFirewall;
pub use cloud::mem::instance::MemInstance;
use failure::Error;
use ipnet::Ipv4AddrRange;
use ipnet::Ipv4Net;
use std::cell::RefCell;
use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::ops::Range;
use std::rc::Rc;
use std::u32;

pub struct MemCloud {
    state: Rc<RefCell<MemCloudState>>,
}

struct MemCloudState {
    ids: Range<u32>,
    ip_addrs: Ipv4AddrRange,
    firewalls: HashMap<String, MemFirewall>,
    instances: HashMap<String, MemInstance>,
}

impl MemCloud {
    pub fn new() -> Result<MemCloud, Error> {
        Ok(MemCloud {
            state: Rc::new(RefCell::new(MemCloudState {
                ids: 0..u32::MAX,
                ip_addrs: Ipv4Net::new(Ipv4Addr::new(0, 0, 0, 0), 0).unwrap().hosts(),
                firewalls: HashMap::new(),
                instances: HashMap::new(),
            })),
        })
    }

    pub fn create_firewall(&self, name: &str) -> Result<MemFirewall, Error> {
        let mut state = self.state.borrow_mut();
        let value = MemFirewall::new(state.fresh_id()?, name.to_owned())?;
        state.firewalls.insert(value.id().to_owned(), value.clone());
        Ok(value)
    }

    pub fn create_instance(
        &self,
        name: &str,
        fqdn: Option<&str>,
        instance_type: &InstanceType,
    ) -> Result<MemInstance, Error> {
        let mut state = self.state.borrow_mut();
        let value = MemInstance::new(
            state.fresh_id()?,
            name.to_owned(),
            fqdn.map(|x| x.to_owned()),
            instance_type.clone(),
            state.fresh_ip_addr()?,
        )?;
        state.instances.insert(value.id().to_owned(), value.clone());
        Ok(value)
    }
}

impl MemCloudState {
    fn fresh_id(&mut self) -> Result<String, Error> {
        self.ids
            .next()
            .map(|id| id.to_string())
            .ok_or_else(|| "exhausted".into())
    }

    fn fresh_ip_addr(&mut self) -> Result<Ipv4Addr, Error> {
        self.ip_addrs.next().ok_or_else(|| "exhausted".into())
    }
}

impl Cloud for MemCloud {
    type Firewall = MemFirewall;
    type Instance = MemInstance;

    fn list_firewalls(&self) -> Result<Vec<MemFirewall>, Error> {
        let state = self.state.borrow();
        let xs = state.firewalls.values().cloned().collect();
        Ok(xs)
    }

    fn list_instances(&self) -> Result<Vec<MemInstance>, Error> {
        let state = self.state.borrow();
        let xs = state.instances.values().cloned().collect();
        Ok(xs)
    }
}
