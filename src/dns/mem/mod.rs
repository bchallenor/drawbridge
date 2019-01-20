mod dns_zone;

pub use crate::dns::mem::dns_zone::MemDnsZone;
use crate::dns::Dns;
use crate::dns::DnsZone;
use failure::Error;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;
use std::u32;

pub struct MemDns {
    state: Rc<RefCell<MemDnsState>>,
}

struct MemDnsState {
    ids: Range<u32>,
    dns_zones: HashMap<String, MemDnsZone>,
}

impl MemDns {
    pub fn new() -> Result<MemDns, Error> {
        Ok(MemDns {
            state: Rc::new(RefCell::new(MemDnsState {
                ids: 0..u32::MAX,
                dns_zones: HashMap::new(),
            })),
        })
    }

    pub fn create_dns_zone(&self, name: &str) -> Result<MemDnsZone, Error> {
        let mut state = self.state.borrow_mut();
        let value = MemDnsZone::new(state.fresh_id()?, name.to_owned())?;
        state.dns_zones.insert(value.id().to_owned(), value.clone());
        Ok(value)
    }
}

impl MemDnsState {
    fn fresh_id(&mut self) -> Result<String, Error> {
        self.ids
            .next()
            .map(|id| id.to_string())
            .ok_or_else(|| format_err!("exhausted"))
    }
}

impl Dns for MemDns {
    type DnsZone = MemDnsZone;

    fn list_zones(&self) -> Result<Vec<MemDnsZone>, Error> {
        let state = self.state.borrow();
        let xs = state.dns_zones.values().cloned().collect();
        Ok(xs)
    }
}
