use dns::DnsTarget;
use dns::DnsZone;
use failure::Error;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

#[derive(Clone)]
pub struct MemDnsZone {
    id: String,
    name: String,
    state: Rc<RefCell<MemDnsZoneState>>,
}

struct MemDnsZoneState {
    records: HashMap<String, DnsTarget>,
}

impl MemDnsZone {
    pub(super) fn new(id: String, name: String) -> Result<MemDnsZone, Error> {
        Ok(MemDnsZone {
            id,
            name,
            state: Rc::new(RefCell::new(MemDnsZoneState {
                records: HashMap::new(),
            })),
        })
    }

    pub fn lookup(&self, fqdn: &str) -> Result<Option<DnsTarget>, Error> {
        let state = self.state.borrow();
        Ok(state.records.get(fqdn).cloned())
    }
}

impl fmt::Debug for MemDnsZone {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} ({})", self.name, self.id)
    }
}

impl DnsZone for MemDnsZone {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn bind(&self, fqdn: &str, target: DnsTarget) -> Result<(), Error> {
        let mut state = self.state.borrow_mut();
        state.records.insert(fqdn.to_owned(), target);
        Ok(())
    }

    fn unbind(&self, fqdn: &str) -> Result<(), Error> {
        let mut state = self.state.borrow_mut();
        state.records.remove(fqdn);
        Ok(())
    }
}
