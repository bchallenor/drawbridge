use crate::cloud::aws::tags::TagFinder;
use crate::cloud::Instance;
use crate::cloud::InstanceRunningState;
use crate::cloud::InstanceType;
use crate::dns::DnsTarget;
use failure::Error;
use failure::ResultExt;
use rusoto_ec2::AttributeValue;
use rusoto_ec2::DescribeInstancesRequest;
use rusoto_ec2::Ec2;
use rusoto_ec2::Filter;
use rusoto_ec2::ModifyInstanceAttributeRequest;
use rusoto_ec2::StartInstancesRequest;
use rusoto_ec2::StopInstancesRequest;
use std::fmt;
use std::net::Ipv4Addr;
use std::rc::Rc;
use std::str::FromStr;
use std::thread;
use std::time::Duration;

pub struct AwsInstance {
    id: String,
    name: String,
    fqdn: Option<String>,
    client: Rc<dyn Ec2>,
}

impl AwsInstance {
    pub(super) fn list(client: &Rc<dyn Ec2>, filter: Filter) -> Result<Vec<AwsInstance>, Error> {
        let req = DescribeInstancesRequest {
            filters: Some(vec![filter]),
            ..Default::default()
        };
        let resp = client
            .describe_instances(&req)
            .sync()
            .with_context(|_e| format!("failed to describe instances: {:?}", req))?;
        let mut values: Vec<AwsInstance> = Vec::new();
        for r in resp.reservations.unwrap() {
            for i in r.instances.unwrap() {
                let id = i.instance_id.unwrap();
                let tags = i.tags.unwrap();
                let name = tags
                    .find_tag("Name")
                    .ok_or_else(|| format_err!("expected instance to have Name tag: {}", id))?;
                let fqdn = tags.find_tag("Fqdn");
                let value = AwsInstance {
                    id: id,
                    name: name.to_owned(),
                    fqdn: fqdn.map(str::to_owned),
                    client: Rc::clone(client),
                };
                values.push(value);
            }
        }
        Ok(values)
    }

    fn get_state(&self) -> Result<InstanceState, Error> {
        let req = DescribeInstancesRequest {
            instance_ids: Some(vec![self.id.clone()]),
            ..Default::default()
        };
        let resp = self
            .client
            .describe_instances(&req)
            .sync()
            .with_context(|_e| format!("failed to describe instance: {:?}", self))?;
        let i = resp
            .reservations
            .unwrap()
            .into_iter()
            .next()
            .and_then(|r| r.instances.unwrap().into_iter().next())
            .ok_or_else(|| format_err!("failed to find instance: {:?}", self))?;
        let instance_state_code = (i.state.unwrap().code.unwrap() as u8).into();
        let instance_type = InstanceType(i.instance_type.unwrap());
        let ebs_optimized = i.ebs_optimized.unwrap();
        let public_ipv4_addr = match i.public_ip_address {
            Some(ip_addr_str) => {
                let ip_addr = Ipv4Addr::from_str(&ip_addr_str)
                    .with_context(|_e| format!("not an IP address: {}", ip_addr_str))?;
                Some(ip_addr)
            }
            None => None,
        };
        let public_dns_name = i.public_dns_name;
        Ok(InstanceState {
            instance_state_code,
            instance_type,
            ebs_optimized,
            public_ipv4_addr,
            public_dns_name,
        })
    }

    fn change_instance_type(&self, instance_type: &InstanceType) -> Result<(), Error> {
        let req = ModifyInstanceAttributeRequest {
            instance_id: self.id.clone(),
            instance_type: Some(AttributeValue {
                value: Some(instance_type.to_string()),
            }),
            ..Default::default()
        };
        self.client
            .modify_instance_attribute(&req)
            .sync()
            .with_context(|_e| {
                format!(
                    "failed to change instance type to {}: {}",
                    instance_type, self.id
                )
            })?;
        Ok(())
    }

    fn request_start(&self) -> Result<(), Error> {
        let req = StartInstancesRequest {
            instance_ids: vec![self.id.clone()],
            ..Default::default()
        };
        self.client
            .start_instances(&req)
            .sync()
            .with_context(|_e| format!("failed to start instance: {}", self.id))?;
        Ok(())
    }

    fn request_stop(&self) -> Result<(), Error> {
        let req = StopInstancesRequest {
            instance_ids: vec![self.id.clone()],
            ..Default::default()
        };
        self.client
            .stop_instances(&req)
            .sync()
            .with_context(|_e| format!("failed to stop instance: {}", self.id))?;
        Ok(())
    }
}

impl fmt::Debug for AwsInstance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.name, self.id)
    }
}

impl Instance for AwsInstance {
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
        let state = self.get_state()?;
        println!("Instance state: {:?}", state);
        if state.instance_type == *instance_type {
            Ok(())
        } else if state.instance_state_code == InstanceStateCode::Stopped {
            self.change_instance_type(instance_type)?;
            Ok(())
        } else {
            Err(format_err!("instance must be stopped to change its type"))
        }
    }

    fn ensure_running(&self) -> Result<InstanceRunningState, Error> {
        loop {
            let state = self.get_state()?;
            println!("Instance state: {:?}", state);
            match state.instance_state_code {
                InstanceStateCode::Pending | InstanceStateCode::Stopping => (),
                InstanceStateCode::Running => {
                    let addr = {
                        if let Some(public_dns_name) = state.public_dns_name {
                            // Prefer the DNS name if it exists,
                            // because AWS will resolve it to an internal IP where possible.
                            Ok(DnsTarget::Cname(public_dns_name))
                        } else if let Some(public_ipv4_addr) = state.public_ipv4_addr {
                            // DNS names are probably disabled for this VPC.
                            // Use the IPv4 address instead.
                            Ok(DnsTarget::A(public_ipv4_addr))
                        } else {
                            Err(format_err!(
                                "expected running instance to have IPv4 address: {:?}",
                                state
                            ))
                        }
                    }?;
                    return Ok(InstanceRunningState {
                        instance_type: state.instance_type,
                        addr,
                    });
                }
                InstanceStateCode::Stopped => self.request_start()?,
                InstanceStateCode::Terminating => bail!("instance is terminating"),
                InstanceStateCode::Terminated => bail!("instance is terminated"),
                InstanceStateCode::Unknown(x) => bail!("instance is in unknown state: {}", x),
            }
            thread::sleep(Duration::from_secs(1));
        }
    }

    fn ensure_stopped(&self) -> Result<(), Error> {
        loop {
            let state = self.get_state()?;
            println!("Instance state: {:?}", state);
            match state.instance_state_code {
                InstanceStateCode::Pending | InstanceStateCode::Stopping => (),
                InstanceStateCode::Running => self.request_stop()?,
                InstanceStateCode::Stopped => return Ok(()),
                InstanceStateCode::Terminating => bail!("instance is terminating"),
                InstanceStateCode::Terminated => bail!("instance is terminated"),
                InstanceStateCode::Unknown(x) => bail!("instance is in unknown state: {}", x),
            }
            thread::sleep(Duration::from_secs(1));
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InstanceState {
    instance_state_code: InstanceStateCode,
    instance_type: InstanceType,
    ebs_optimized: bool,
    public_ipv4_addr: Option<Ipv4Addr>,
    public_dns_name: Option<String>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum InstanceStateCode {
    Pending,
    Running,
    Terminating, // called "shutting-down" by AWS
    Terminated,
    Stopping,
    Stopped,
    Unknown(u8),
}

// TODO: probably should be TryFrom, without the Unknown state
impl From<u8> for InstanceStateCode {
    fn from(code: u8) -> InstanceStateCode {
        match code {
            0 => InstanceStateCode::Pending,
            16 => InstanceStateCode::Running,
            32 => InstanceStateCode::Terminating,
            48 => InstanceStateCode::Terminated,
            64 => InstanceStateCode::Stopping,
            80 => InstanceStateCode::Stopped,
            x => InstanceStateCode::Unknown(x),
        }
    }
}
