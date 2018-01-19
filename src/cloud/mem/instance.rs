use cloud::Instance;
use cloud::InstanceRunningState;
use cloud::InstanceType;
use errors::*;
use std::cell::RefCell;
use std::fmt;
use std::net::Ipv4Addr;
use std::rc::Rc;

#[derive(Clone)]
pub struct MemInstance {
    id: String,
    name: String,
    fqdn: Option<String>,
    ip_addr_when_running: Ipv4Addr,
    state: Rc<RefCell<MemInstanceState>>,
}

struct MemInstanceState {
    running_state: Option<InstanceRunningState>,
}

impl MemInstance {
    pub(super) fn new(
        id: String,
        name: String,
        fqdn: Option<String>,
        ip_addr_when_running: Ipv4Addr,
    ) -> Result<MemInstance> {
        Ok(MemInstance {
            id,
            name,
            fqdn,
            ip_addr_when_running,
            state: Rc::new(RefCell::new(MemInstanceState {
                running_state: None,
            })),
        })
    }

    pub fn try_get_running_state(&self) -> Result<Option<InstanceRunningState>> {
        let state = self.state.borrow();
        Ok(state.running_state.clone())
    }
}

impl fmt::Debug for MemInstance {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} ({})", self.name, self.id)
    }
}

impl Instance for MemInstance {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn fqdn(&self) -> Option<&str> {
        self.fqdn.as_ref().map(String::as_ref)
    }

    fn ensure_running(&self, instance_type: &InstanceType) -> Result<InstanceRunningState> {
        let mut state = self.state.borrow_mut();
        let running_state = InstanceRunningState {
            instance_type: instance_type.clone(),
            ip_addr: self.ip_addr_when_running,
        };
        state.running_state = Some(running_state.clone()); // TODO: all the clones!
        Ok(running_state)
    }

    fn ensure_stopped(&self) -> Result<()> {
        let mut state = self.state.borrow_mut();
        state.running_state = None;
        Ok(())
    }
}
