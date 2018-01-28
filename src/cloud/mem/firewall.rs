use cloud::Firewall;
use failure::Error;
use iprules::IpIngressRule;
use std::cell::RefCell;
use std::collections::HashSet;
use std::fmt;
use std::rc::Rc;

#[derive(Clone)]
pub struct MemFirewall {
    id: String,
    name: String,
    state: Rc<RefCell<MemFirewallState>>,
}

struct MemFirewallState {
    rules: HashSet<IpIngressRule>,
}

impl MemFirewall {
    pub(super) fn new(id: String, name: String) -> Result<MemFirewall> {
        Ok(MemFirewall {
            id,
            name,
            state: Rc::new(RefCell::new(MemFirewallState {
                rules: HashSet::new(),
            })),
        })
    }
}

impl fmt::Debug for MemFirewall {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} ({})", self.name, self.id)
    }
}

impl Firewall for MemFirewall {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn list_ingress_rules(&self) -> Result<HashSet<IpIngressRule>> {
        let state = self.state.borrow();
        Ok(state.rules.iter().cloned().collect())
    }

    fn add_ingress_rules<'a, R>(&self, rules: R) -> Result<()>
    where
        R: IntoIterator<Item = &'a IpIngressRule>,
    {
        let mut state = self.state.borrow_mut();
        for rule in rules {
            state.rules.insert(*rule);
        }
        Ok(())
    }

    fn remove_ingress_rules<'a, R>(&self, rules: R) -> Result<()>
    where
        R: IntoIterator<Item = &'a IpIngressRule>,
    {
        let mut state = self.state.borrow_mut();
        for rule in rules {
            state.rules.remove(rule);
        }
        Ok(())
    }
}
