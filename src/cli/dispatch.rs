use crate::cli::Command;
use crate::cloud::Cloud;
use crate::cloud::Firewall;
use crate::cloud::Instance;
use crate::dns::Dns;
use crate::dns::DnsTarget;
use crate::dns::DnsZone;
use crate::iprules::IpIngressRule;
use failure::Error;
use std::collections::HashSet;

pub fn dispatch<C, D>(cmd: Command, cloud: &C, dns: &D) -> Result<(), Error>
where
    C: Cloud,
    D: Dns,
{
    println!("Running command: {:?}", cmd);

    match cmd {
        Command::Open {
            ref ip_cidrs,
            ref ip_protocols,
            ref names,
        } => {
            let desired_rules = {
                let mut ip_rules = HashSet::new();
                for ip_cidr in ip_cidrs {
                    for ip_protocol in ip_protocols {
                        ip_rules.insert(IpIngressRule(*ip_cidr, *ip_protocol));
                    }
                }
                ip_rules
            };

            let fws = cloud.list_firewalls(names)?;
            println!("Found firewalls: {:?}", fws);

            for fw in fws {
                println!("Opening firewall: {:?}", fw);
                sync_firewall_rules(fw, &desired_rules)?;
            }
        }
        Command::Close { ref names } => {
            let desired_rules = HashSet::new();

            let fws = cloud.list_firewalls(names)?;
            println!("Found firewalls: {:?}", fws);

            for fw in fws {
                println!("Closing firewall: {:?}", fw);
                sync_firewall_rules(fw, &desired_rules)?;
            }
        }
        Command::Start {
            ref instance_type,
            ref names,
        } => {
            let instances = cloud.list_instances(names)?;
            println!("Found instances: {:?}", instances);

            for instance in instances {
                println!("Starting instance: {:?}", instance);

                if let &Some(ref instance_type) = instance_type {
                    instance.try_ensure_instance_type(instance_type)?;
                }
                let state = instance.ensure_running()?;
                println!(
                    "Instance running with type: {} and address: {:?}",
                    state.instance_type, state.addr
                );

                if let Some(fqdn) = instance.fqdn() {
                    sync_dns(dns, fqdn, Some(state.addr))?;
                }
            }
        }
        Command::Stop { ref names } => {
            let instances = cloud.list_instances(names)?;
            println!("Found instances: {:?}", instances);

            for instance in instances {
                println!("Stopping instance: {:?}", instance);

                // Unbind DNS before stopping
                if let Some(fqdn) = instance.fqdn() {
                    sync_dns(dns, fqdn, None)?;
                }

                instance.ensure_stopped()?;
                println!("Instance stopped");
            }
        }
    };

    Ok(())
}

fn sync_firewall_rules<F>(fw: F, desired_rules: &HashSet<IpIngressRule>) -> Result<(), Error>
where
    F: Firewall,
{
    println!("Desired rules: {:?}", desired_rules);

    let existing_rules = fw.list_ingress_rules()?;
    println!("Existing rules: {:?}", existing_rules);

    let missing_rules = desired_rules - &existing_rules;
    println!("Adding rules: {:?}", missing_rules);
    fw.add_ingress_rules(&missing_rules)?;

    let extra_rules = &existing_rules - desired_rules;
    println!("Removing rules: {:?}", extra_rules);
    fw.remove_ingress_rules(&extra_rules)?;

    Ok(())
}

fn sync_dns<D>(dns: &D, fqdn: &str, target_or_none: Option<DnsTarget>) -> Result<(), Error>
where
    D: Dns,
{
    let dns_zone = dns.find_authoritative_zone(fqdn)?;
    println!("Found authoritative DNS zone for {}: {:?}", fqdn, dns_zone);

    if let Some(target) = target_or_none {
        dns_zone.bind(fqdn, target)?;
        println!("Bound hostname: {}", fqdn);
    } else {
        dns_zone.unbind(fqdn)?;
        println!("Unbound hostname: {}", fqdn);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud::mem::MemCloud;
    use crate::cloud::mem::MemInstance;
    use crate::cloud::InstanceType;
    use crate::dns::mem::MemDns;
    use crate::iprules::IpProtocol;
    use ipnet::IpNet;

    // TODO(ques_in_main)

    #[test]
    fn test_open_firewall_with_no_existing_rules() {
        test_open_firewall(
            &[],
            &["1.1.0.0/16".parse().unwrap(), "9.9.9.9/32".parse().unwrap()],
            &["22/tcp".parse().unwrap(), "80/tcp".parse().unwrap()],
        )
        .unwrap();
    }

    #[test]
    fn test_open_firewall_with_some_existing_rules() {
        test_open_firewall(
            &[
                // wrong protocol: will be removed
                IpIngressRule("9.9.9.9/32".parse().unwrap(), "443/tcp".parse().unwrap()),
                // wrong CIDR: will be removed
                IpIngressRule("1.1.1.1/32".parse().unwrap(), "80/tcp".parse().unwrap()),
                // correct: will be preserved
                IpIngressRule("1.1.0.0/16".parse().unwrap(), "22/tcp".parse().unwrap()),
            ],
            &["1.1.0.0/16".parse().unwrap(), "9.9.9.9/32".parse().unwrap()],
            &["22/tcp".parse().unwrap(), "80/tcp".parse().unwrap()],
        )
        .unwrap();
    }

    fn test_open_firewall(
        existing_rules: &[IpIngressRule],
        ip_cidrs: &[IpNet],
        ip_protocols: &[IpProtocol],
    ) -> Result<(), Error> {
        let mut expected_rules = HashSet::new();
        for ip_cidr in ip_cidrs {
            for ip_protocol in ip_protocols {
                expected_rules.insert(IpIngressRule(*ip_cidr, *ip_protocol));
            }
        }

        let cloud = MemCloud::new()?;
        let fw = cloud.create_firewall("fw")?;
        fw.add_ingress_rules(existing_rules)?;

        let dns = MemDns::new()?;

        let cmd = Command::Open {
            ip_cidrs: ip_cidrs.to_vec(),
            ip_protocols: ip_protocols.to_vec(),
            names: vec!["fw".to_owned()],
        };

        // test that open command opens the firewall
        dispatch(cmd, &cloud, &dns)?;

        assert_eq!(expected_rules, fw.list_ingress_rules()?);

        // test that close command closes the firewall, and that it is idempotent
        for _ in 0..2 {
            dispatch(
                Command::Close {
                    names: vec!["fw".to_owned()],
                },
                &cloud,
                &dns,
            )?;

            assert_eq!(HashSet::new(), fw.list_ingress_rules()?);
        }

        Ok(())
    }

    #[test]
    fn test_start_instance_that_is_stopped() {
        test_start_instance(
            |cloud| {
                let inst = cloud.create_instance("inst", None, &InstanceType::new("t2.medium"))?;
                inst.ensure_stopped()?;
                Ok(inst)
            },
            None,
        )
        .unwrap();
    }

    #[test]
    fn test_start_instance_that_is_stopped_with_other_instance_type() {
        test_start_instance(
            |cloud| {
                let inst = cloud.create_instance("inst", None, &InstanceType::new("t2.medium"))?;
                inst.ensure_stopped()?;
                Ok(inst)
            },
            Some(InstanceType::new("t2.large")),
        )
        .unwrap();
    }

    #[test]
    fn test_start_instance_that_is_already_started() {
        test_start_instance(
            |cloud| {
                let inst = cloud.create_instance("inst", None, &InstanceType::new("t2.medium"))?;
                inst.ensure_running()?;
                Ok(inst)
            },
            None,
        )
        .unwrap();
    }

    #[test]
    fn test_start_instance_that_is_already_started_with_other_instance_type() {
        let err = test_start_instance(
            |cloud| {
                let inst = cloud.create_instance("inst", None, &InstanceType::new("t2.medium"))?;
                inst.ensure_running()?;
                Ok(inst)
            },
            Some(InstanceType::new("t2.large")),
        )
        .unwrap_err();
        assert_eq!(
            "instance must be stopped to change its type",
            err.to_string()
        );
    }

    fn test_start_instance<F>(
        instance_builder: F,
        instance_type: Option<InstanceType>,
    ) -> Result<(), Error>
    where
        F: FnOnce(&MemCloud) -> Result<MemInstance, Error>,
    {
        let cloud = MemCloud::new()?;
        let inst = instance_builder(&cloud)?;

        let dns = MemDns::new()?;

        let cmd = Command::Start {
            instance_type: instance_type.clone(),
            names: vec!["inst".to_owned()],
        };

        // test that start command starts the instance
        dispatch(cmd, &cloud, &dns)?;

        let running_state = inst.try_get_running_state()?;
        assert_eq!(true, running_state.is_some()); // i.e. running
        if let &Some(ref instance_type) = &instance_type {
            assert_eq!(*instance_type, running_state.unwrap().instance_type);
        }

        // test that stop command stops the instance, and that it is idempotent
        for _ in 0..2 {
            dispatch(
                Command::Stop {
                    names: vec!["inst".to_owned()],
                },
                &cloud,
                &dns,
            )?;

            let running_state = inst.try_get_running_state()?;
            assert_eq!(true, running_state.is_none()); // i.e. stopped
        }

        Ok(())
    }

    #[test]
    fn test_bind_simple_hostname_to_root_zone() {
        test_bind_dns(
            "inst.example.com",
            // should bind to this
            "example.com",
            // and not to any of these
            &["other.example.com", "example.net"],
        )
        .unwrap();
    }

    #[test]
    fn test_bind_complex_hostname_to_root_zone() {
        test_bind_dns(
            "inst.sub.example.com",
            // should bind to this
            "example.com",
            // and not to any of these
            &["other.example.com", "example.net"],
        )
        .unwrap();
    }

    #[test]
    fn test_bind_complex_hostname_to_sub_zone() {
        test_bind_dns(
            "inst.sub.example.com",
            // should bind to this
            "sub.example.com",
            // and not to any of these
            &["example.com", "other.example.com", "example.net"],
        )
        .unwrap();
    }

    fn test_bind_dns(
        inst_fqdn: &str,
        zone_fqdn: &str,
        other_zone_fqdns: &[&str],
    ) -> Result<(), Error> {
        let cloud = MemCloud::new()?;
        let inst =
            cloud.create_instance("inst", Some(inst_fqdn), &InstanceType::new("t2.medium"))?;

        let dns = MemDns::new()?;
        let zone = dns.create_dns_zone(zone_fqdn)?;
        let other_zones = other_zone_fqdns
            .iter()
            .map(|fqdn| dns.create_dns_zone(fqdn))
            .collect::<Result<Vec<_>, Error>>()?;

        let cmd = Command::Start {
            instance_type: None,
            names: vec!["inst".to_owned()],
        };

        // test that start command binds the DNS
        dispatch(cmd, &cloud, &dns)?;

        let running_state = inst.try_get_running_state()?;
        assert_eq!(true, running_state.is_some()); // i.e. running
        assert_eq!(Some(running_state.unwrap().addr), zone.lookup(inst_fqdn)?);
        for other_zone in &other_zones {
            assert_eq!(None, other_zone.lookup(inst_fqdn)?);
        }

        // test that stop command unbinds the DNS, and that it is idempotent
        for _ in 0..2 {
            dispatch(
                Command::Stop {
                    names: vec!["inst".to_owned()],
                },
                &cloud,
                &dns,
            )?;

            assert_eq!(None, zone.lookup(inst_fqdn)?);
            for other_zone in &other_zones {
                assert_eq!(None, other_zone.lookup(inst_fqdn)?);
            }
        }

        Ok(())
    }
}
