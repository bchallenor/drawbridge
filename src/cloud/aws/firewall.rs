use cloud::Firewall;
use errors::*;
use ipnet::IpNet;
use ipnet::Ipv4Net;
use ipnet::Ipv6Net;
use iprules::IpIngressRule;
use iprules::IpPortRange;
use iprules::IpProtocol;
use rusoto_ec2::AuthorizeSecurityGroupIngressRequest;
use rusoto_ec2::DescribeSecurityGroupsRequest;
use rusoto_ec2::Ec2;
use rusoto_ec2::Filter;
use rusoto_ec2::IpPermission;
use rusoto_ec2::IpRange;
use rusoto_ec2::Ipv6Range;
use rusoto_ec2::RevokeSecurityGroupIngressRequest;
use rusoto_ec2::SecurityGroup;
use std::collections::HashSet;
use std::fmt;
use std::rc::Rc;
use std::str::FromStr;

pub struct AwsFirewall {
    id: String,
    name: String,
    client: Rc<Ec2>,
}

impl AwsFirewall {
    pub(super) fn list(client: &Rc<Ec2>, filter: &Filter) -> Result<Vec<AwsFirewall>> {
        let req = DescribeSecurityGroupsRequest {
            filters: Some(vec![filter.clone()]),
            ..Default::default()
        };
        let resp = client
            .describe_security_groups(&req)
            .chain_err(|| format!("failed to describe security groups: {:?}", req))?;
        let mut values: Vec<AwsFirewall> = Vec::new();
        for sg in resp.security_groups.unwrap() {
            let value = AwsFirewall {
                id: sg.group_id.unwrap(),
                name: sg.group_name.unwrap(),
                client: Rc::clone(client),
            };
            values.push(value);
        }
        Ok(values)
    }

    fn get_state(&self) -> Result<SecurityGroup> {
        let req = DescribeSecurityGroupsRequest {
            group_ids: Some(vec![self.id.clone()]),
            ..Default::default()
        };
        let resp = self.client
            .describe_security_groups(&req)
            .chain_err(|| format!("failed to describe security group: {:?}", self))?;
        resp.security_groups
            .unwrap()
            .into_iter()
            .next()
            .ok_or_else(|| format!("failed to find security group: {:?}", self).into())
    }
}

impl fmt::Debug for AwsFirewall {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} ({})", self.name, self.id)
    }
}

impl Firewall for AwsFirewall {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn list_ingress_rules(&self) -> Result<HashSet<IpIngressRule>> {
        let mut rules = HashSet::new();

        let sg = self.get_state()?;
        for ip_permission in sg.ip_permissions.unwrap() {
            let ip_port_range = IpPortRange(
                ip_permission.from_port.unwrap() as u16,
                ip_permission.to_port.unwrap() as u16,
            );
            let ip_protocol = match ip_permission.ip_protocol.unwrap().as_ref() {
                "tcp" => IpProtocol::Tcp(ip_port_range),
                "udp" => IpProtocol::Udp(ip_port_range),
                x => return Err(format!("unknown protocol: {}", x).into()),
            };
            for ip_range in ip_permission.ip_ranges.unwrap() {
                let ip_cidr_str = &ip_range.cidr_ip.unwrap();
                let ip_cidr = Ipv4Net::from_str(ip_cidr_str)
                    .chain_err(|| format!("not an IPv4 network: {}", ip_cidr_str))?;
                rules.insert(IpIngressRule(IpNet::V4(ip_cidr), ip_protocol));
            }
            for ip_range in ip_permission.ipv_6_ranges.unwrap() {
                let ip_cidr_str = &ip_range.cidr_ipv_6.unwrap();
                let ip_cidr = Ipv6Net::from_str(ip_cidr_str)
                    .chain_err(|| format!("not an IPv6 network: {}", ip_cidr_str))?;
                rules.insert(IpIngressRule(IpNet::V6(ip_cidr), ip_protocol));
            }
        }

        Ok(rules)
    }

    fn add_ingress_rules<'a, R>(&self, rules: R) -> Result<()>
    where
        R: IntoIterator<Item = &'a IpIngressRule>,
    {
        let ip_permissions: Vec<IpPermission> = rules.into_iter().map(to_ip_permission).collect();
        if ip_permissions.is_empty() {
            return Ok(());
        }
        let req = AuthorizeSecurityGroupIngressRequest {
            group_id: Some(self.id.clone()),
            ip_permissions: Some(ip_permissions),
            ..Default::default()
        };
        self.client
            .authorize_security_group_ingress(&req)
            .chain_err(|| {
                format!(
                    "failed to authorize ingress for security group: {}",
                    self.name
                )
            })?;
        Ok(())
    }

    fn remove_ingress_rules<'a, R>(&self, rules: R) -> Result<()>
    where
        R: IntoIterator<Item = &'a IpIngressRule>,
    {
        let ip_permissions: Vec<IpPermission> = rules.into_iter().map(to_ip_permission).collect();
        if ip_permissions.is_empty() {
            return Ok(());
        }
        let req = RevokeSecurityGroupIngressRequest {
            group_id: Some(self.id.clone()),
            ip_permissions: Some(ip_permissions),
            ..Default::default()
        };
        self.client
            .revoke_security_group_ingress(&req)
            .chain_err(|| format!("failed to revoke ingress for security group: {}", self.name))?;
        Ok(())
    }
}

fn to_ip_permission(rule: &IpIngressRule) -> IpPermission {
    let &IpIngressRule(ref ip_cidr, ref ip_protocol) = rule;
    let (ip_protocol, from_port, to_port) = match ip_protocol {
        &IpProtocol::Tcp(IpPortRange(from, to)) => ("tcp", from, to),
        &IpProtocol::Udp(IpPortRange(from, to)) => ("udp", from, to),
    };
    let (ip_ranges, ipv_6_ranges) = match *ip_cidr {
        IpNet::V4(ipv4_cidr) => (
            Some(vec![
                IpRange {
                    cidr_ip: Some(ipv4_cidr.to_string()),
                },
            ]),
            None,
        ),
        IpNet::V6(ipv6_cidr) => (
            None,
            Some(vec![
                Ipv6Range {
                    cidr_ipv_6: Some(ipv6_cidr.to_string()),
                },
            ]),
        ),
    };
    IpPermission {
        ip_protocol: Some(ip_protocol.to_owned()),
        from_port: Some(from_port.into()),
        to_port: Some(to_port.into()),
        ip_ranges: ip_ranges,
        ipv_6_ranges: ipv_6_ranges,
        prefix_list_ids: None,
        user_id_group_pairs: None,
    }
}
