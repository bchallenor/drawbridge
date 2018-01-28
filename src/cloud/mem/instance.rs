use cloud::Instance;
use cloud::InstanceRunningState;
use cloud::InstanceType;
use failure::Error;
use std::cell::RefCell;
use std::fmt;
use std::net::Ipv4Addr;
use std::rc::Rc;

#[derive(Clone)]
pub struct MemInstance {
    id: String,
    name: String,
    fqdn: Option<String>,
    state: Rc<RefCell<MemInstanceState>>,
}

struct MemInstanceState {
    instance_type: InstanceType,
    ip_addr: Ipv4Addr,
    is_running: bool,
}

impl MemInstance {
    pub(super) fn new(
        id: String,
        name: String,
        fqdn: Option<String>,
        instance_type: InstanceType,
        ip_addr: Ipv4Addr,
    ) -> Result<MemInstance, Error> {
        Ok(MemInstance {
            id,
            name,
            fqdn,
            state: Rc::new(RefCell::new(MemInstanceState {
                instance_type,
                ip_addr,
                is_running: false,
            })),
        })
    }

    pub fn try_get_running_state(&self) -> Result<Option<InstanceRunningState>, Error> {
        let state = self.state.borrow();
        if state.is_running {
            Ok(Some(InstanceRunningState {
                instance_type: state.instance_type.clone(),
                ip_addr: state.ip_addr,
            }))
        } else {
            Ok(None)
        }
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

    fn try_ensure_instance_type(&self, instance_type: &InstanceType) -> Result<(), Error> {
        let mut state = self.state.borrow_mut();
        if state.instance_type == *instance_type {
            Ok(())
        } else if !state.is_running {
            state.instance_type = instance_type.clone();
            Ok(())
        } else {
            Err("instance must be stopped to change its type".into())
        }
    }

    fn ensure_running(&self) -> Result<InstanceRunningState, Error> {
        let mut state = self.state.borrow_mut();
        let running_state = InstanceRunningState {
            instance_type: state.instance_type.clone(),
            ip_addr: state.ip_addr,
        };
        state.is_running = true;
        Ok(running_state)
    }

    fn ensure_stopped(&self) -> Result<(), Error> {
        let mut state = self.state.borrow_mut();
        state.is_running = false;
        Ok(())
    }
}
