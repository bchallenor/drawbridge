#[macro_use]
extern crate error_chain;
extern crate ipnet;
extern crate openssl_probe;
extern crate rusoto_core;
extern crate rusoto_ec2;
extern crate rusoto_route53;

mod cloud;
mod dns;
mod errors;
mod iprules;

use cloud::Cloud;
use cloud::Firewall;
use cloud::Instance;
use cloud::InstanceType;
use cloud::aws::AwsCloud;
use dns::Dns;
use dns::DnsZone;
use dns::aws::AwsDns;
use errors::*;
use ipnet::Ipv4Net;
use iprules::IpIngressRule;
use iprules::IpService;
use std::collections::HashSet;
use std::env;
use std::str::FromStr;

quick_main!(run);

enum Command {
    Start {
        instance_type: InstanceType,
        ip_cidrs: Vec<Ipv4Net>,
        ip_services: Vec<IpService>,
    },
    Stop,
}

fn run() -> Result<()> {
    // For e.g. Termux support on Android
    openssl_probe::init_ssl_cert_env_vars();

    let mut args = env::args().skip(1);

    let cmd = match args.next() {
        Some(instance_type_str) => {
            let instance_type = InstanceType(instance_type_str);

            let ip_cidrs = args.map(|x| {
                Ipv4Net::from_str(&x).chain_err(|| format!("not a CIDR network: {}", &x))
            }).collect::<Result<Vec<_>>>()?;

            let ip_services = vec![
                "22/tcp".parse().unwrap(),
                "60000-61000/udp".parse().unwrap(),
            ];

            Command::Start {
                instance_type,
                ip_cidrs,
                ip_services,
            }
        }
        None => Command::Stop,
    };

    let profile_opt = std::env::var("DRAWBRIDGE_PROFILE").ok();
    let profile = profile_opt.as_ref().map_or("default", String::as_ref);

    let tag_key = "Drawbridge";
    let tag_value = profile;
    println!("Filtering resources with tag: {}={}", tag_key, tag_value);

    let cloud = AwsCloud::new(tag_key, tag_value)?;
    let dns = AwsDns::new()?;

    dispatch(cmd, &cloud, &dns)
}

fn dispatch<C, D>(cmd: Command, cloud: &C, dns: &D) -> Result<()>
where
    C: Cloud,
    D: Dns,
{
    let fws = cloud.list_firewalls()?;
    println!("Found firewalls: {:?}", fws);

    let instances = cloud.list_instances()?;
    println!("Found instances: {:?}", instances);

    let desired_rules = match cmd {
        Command::Start {
            ref ip_cidrs,
            ref ip_services,
            ..
        } => {
            let mut ip_rules = HashSet::new();
            for ip_cidr in ip_cidrs {
                for ip_service in ip_services {
                    ip_rules.insert(IpIngressRule(*ip_cidr, *ip_service));
                }
            }
            ip_rules
        }
        Command::Stop => HashSet::new(),
    };

    for fw in fws {
        println!("Processing firewall: {:?}", fw);

        let existing_rules = fw.list_ingress_rules()?;
        println!("Existing rules: {:?}", existing_rules);

        let missing_rules = &desired_rules - &existing_rules;
        println!("Adding rules: {:?}", missing_rules);
        fw.add_ingress_rules(&missing_rules)?;

        let extra_rules = &existing_rules - &desired_rules;
        println!("Removing rules: {:?}", extra_rules);
        fw.remove_ingress_rules(&extra_rules)?;
    }

    for instance in instances {
        println!("Processing instance: {:?}", instance);

        let ip_addr_or_none = match cmd {
            Command::Start {
                ref instance_type, ..
            } => {
                let state = instance.ensure_running(instance_type)?;
                println!(
                    "Instance running with type: {} and IP address: {}",
                    state.instance_type, state.ip_addr
                );
                Some(state.ip_addr)
            }
            Command::Stop => {
                instance.ensure_stopped()?;
                println!("Instance stopped");
                None
            }
        };

        if let Some(fqdn) = instance.fqdn() {
            let dns_zone = dns.find_authoritative_zone(fqdn)?;
            println!("Found authoritative DNS zone for {}: {:?}", fqdn, dns_zone);

            if let Some(ip_addr) = ip_addr_or_none {
                dns_zone.bind(fqdn, ip_addr)?;
                println!("Bound hostname: {}", fqdn);
            } else {
                dns_zone.unbind(fqdn)?;
                println!("Unbound hostname: {}", fqdn);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cloud::mem::MemCloud;
    use cloud::mem::MemInstance;
    use dns::mem::MemDns;

    // TODO(ques_in_main)

    #[test]
    fn test_open_firewall_with_no_existing_rules() {
        test_open_firewall(
            &[],
            &["1.1.0.0/16".parse().unwrap(), "9.9.9.9/32".parse().unwrap()],
            &["22/tcp".parse().unwrap(), "80/tcp".parse().unwrap()],
        ).unwrap();
    }

    #[test]
    fn test_open_firewall_with_some_existing_rules() {
        test_open_firewall(
            &[
                // wrong service: will be removed
                IpIngressRule("9.9.9.9/32".parse().unwrap(), "443/tcp".parse().unwrap()),
                // wrong CIDR: will be removed
                IpIngressRule("1.1.1.1/32".parse().unwrap(), "80/tcp".parse().unwrap()),
                // correct: will be preserved
                IpIngressRule("1.1.0.0/16".parse().unwrap(), "22/tcp".parse().unwrap()),
            ],
            &["1.1.0.0/16".parse().unwrap(), "9.9.9.9/32".parse().unwrap()],
            &["22/tcp".parse().unwrap(), "80/tcp".parse().unwrap()],
        ).unwrap();
    }

    fn test_open_firewall(
        existing_rules: &[IpIngressRule],
        ip_cidrs: &[Ipv4Net],
        ip_services: &[IpService],
    ) -> Result<()> {
        let mut expected_rules = HashSet::new();
        for ip_cidr in ip_cidrs {
            for ip_service in ip_services {
                expected_rules.insert(IpIngressRule(*ip_cidr, *ip_service));
            }
        }

        let cloud = MemCloud::new()?;
        let fw = cloud.create_firewall("fw")?;
        fw.add_ingress_rules(existing_rules)?;

        let dns = MemDns::new()?;

        let cmd = Command::Start {
            instance_type: InstanceType("t2.medium".to_owned()),
            ip_cidrs: ip_cidrs.to_vec(),
            ip_services: ip_services.to_vec(),
        };

        // test that start command opens the firewall
        dispatch(cmd, &cloud, &dns)?;

        assert_eq!(expected_rules, fw.list_ingress_rules()?);

        // test that stop command closes the firewall, and that it is idempotent
        for _ in 0..2 {
            dispatch(Command::Stop, &cloud, &dns)?;

            assert_eq!(HashSet::new(), fw.list_ingress_rules()?);
        }

        Ok(())
    }

    #[test]
    fn test_start_instance_that_is_stopped() {
        test_start_instance(
            |cloud| {
                let inst = cloud.create_instance("inst", None)?;
                inst.ensure_stopped()?;
                Ok(inst)
            },
            InstanceType("t2.medium".to_owned()),
        ).unwrap();
    }

    #[test]
    fn test_start_instance_that_is_already_started() {
        test_start_instance(
            |cloud| {
                let inst = cloud.create_instance("inst", None)?;
                inst.ensure_running(&InstanceType("t2.medium".to_owned()))?;
                Ok(inst)
            },
            InstanceType("t2.medium".to_owned()),
        ).unwrap();
    }

    #[test]
    fn test_start_instance_that_is_already_started_with_other_instance_type() {
        test_start_instance(
            |cloud| {
                let inst = cloud.create_instance("inst", None)?;
                inst.ensure_running(&InstanceType("t2.medium".to_owned()))?;
                Ok(inst)
            },
            InstanceType("t2.large".to_owned()),
        ).unwrap();
    }

    fn test_start_instance<F>(instance_builder: F, instance_type: InstanceType) -> Result<()>
    where
        F: FnOnce(&MemCloud) -> Result<MemInstance>,
    {
        let cloud = MemCloud::new()?;
        let inst = instance_builder(&cloud)?;

        let dns = MemDns::new()?;

        let cmd = Command::Start {
            instance_type: instance_type.clone(),
            ip_cidrs: vec![],
            ip_services: vec![],
        };

        // test that start command starts the instance
        dispatch(cmd, &cloud, &dns)?;

        let running_state = inst.try_get_running_state()?;
        assert_eq!(true, running_state.is_some()); // i.e. running
        assert_eq!(instance_type, running_state.unwrap().instance_type);

        // test that stop command stops the instance, and that it is idempotent
        for _ in 0..2 {
            dispatch(Command::Stop, &cloud, &dns)?;

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
        ).unwrap();
    }

    #[test]
    fn test_bind_complex_hostname_to_root_zone() {
        test_bind_dns(
            "inst.sub.example.com",
            // should bind to this
            "example.com",
            // and not to any of these
            &["other.example.com", "example.net"],
        ).unwrap();
    }

    #[test]
    fn test_bind_complex_hostname_to_sub_zone() {
        test_bind_dns(
            "inst.sub.example.com",
            // should bind to this
            "sub.example.com",
            // and not to any of these
            &["example.com", "other.example.com", "example.net"],
        ).unwrap();
    }

    fn test_bind_dns(inst_fqdn: &str, zone_fqdn: &str, other_zone_fqdns: &[&str]) -> Result<()> {
        let cloud = MemCloud::new()?;
        let inst = cloud.create_instance("inst", Some(inst_fqdn))?;

        let dns = MemDns::new()?;
        let zone = dns.create_dns_zone(zone_fqdn)?;
        let other_zones = other_zone_fqdns
            .iter()
            .map(|fqdn| dns.create_dns_zone(fqdn))
            .collect::<Result<Vec<_>>>()?;

        let cmd = Command::Start {
            instance_type: InstanceType("t2.medium".to_owned()),
            ip_cidrs: vec![],
            ip_services: vec![],
        };

        // test that start command binds the DNS
        dispatch(cmd, &cloud, &dns)?;

        let running_state = inst.try_get_running_state()?;
        assert_eq!(true, running_state.is_some()); // i.e. running
        assert_eq!(
            Some(running_state.unwrap().ip_addr),
            zone.lookup(inst_fqdn)?
        );
        for other_zone in &other_zones {
            assert_eq!(None, other_zone.lookup(inst_fqdn)?);
        }

        // test that stop command unbinds the DNS, and that it is idempotent
        for _ in 0..2 {
            dispatch(Command::Stop, &cloud, &dns)?;

            assert_eq!(None, zone.lookup(inst_fqdn)?);
            for other_zone in &other_zones {
                assert_eq!(None, other_zone.lookup(inst_fqdn)?);
            }
        }

        Ok(())
    }
}
