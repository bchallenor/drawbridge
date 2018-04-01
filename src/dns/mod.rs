pub mod aws;
#[cfg(test)]
pub mod mem;

use failure::Error;
use std::fmt;
use std::net::Ipv4Addr;
use std::str;

pub trait Dns {
    type DnsZone: DnsZone;

    fn list_zones(&self) -> Result<Vec<Self::DnsZone>, Error>;

    fn find_authoritative_zone(&self, name: &str) -> Result<Self::DnsZone, Error> {
        let parts: Vec<&str> = name.split_terminator('.').collect();
        let zones = self.list_zones()?;
        zones
            .into_iter()
            .filter(|zone| {
                let zone_parts: Vec<&str> = zone.name().split_terminator('.').collect();
                parts.ends_with(&zone_parts)
            })
            .max_by_key(|zone| zone.name().len())
            .ok_or_else(|| format_err!("could not find authoritative DNS zone for: {}", name))
    }
}

pub trait DnsZone: fmt::Debug {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn bind(&self, fqdn: &str, target: DnsTarget) -> Result<(), Error>;
    fn unbind(&self, fqdn: &str) -> Result<(), Error>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DnsTarget {
    A(Ipv4Addr),
    // TODO: Aaaa(Ipv6Addr),
    Cname(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO(ques_in_main)

    #[test]
    fn test_find_authoritative_zone() {
        test_find_authoritative_zone_impl().unwrap();
    }

    fn test_find_authoritative_zone_impl() -> Result<(), Error> {
        let dns = TestDns {
            zones: vec![
                TestDnsZone::new("example.com"),
                TestDnsZone::new("sub1.example.com"),
                TestDnsZone::new("sub2.example.com"),
                TestDnsZone::new("unrelated.sub1.example.com"),
                TestDnsZone::new("unrelated.sub2.example.com"),
                TestDnsZone::new("unrelated.sub3.example.com"),
                TestDnsZone::new("unrelated-sub1.example.com"),
                TestDnsZone::new("unrelated-sub2.example.com"),
                TestDnsZone::new("unrelated-sub3.example.com"),
                TestDnsZone::new("example.net"),
            ],
        };

        assert_eq!(
            "example.com",
            dns.find_authoritative_zone("x.example.com")?.name()
        );
        assert_eq!(
            "sub1.example.com",
            dns.find_authoritative_zone("x.sub1.example.com")?.name()
        );
        assert_eq!(
            "sub2.example.com",
            dns.find_authoritative_zone("x.sub2.example.com")?.name()
        );
        // There is no sub3.example.com, so example.com is authoritative
        assert_eq!(
            "example.com",
            dns.find_authoritative_zone("x.sub3.example.com")?.name()
        );

        Ok(())
    }

    #[derive(Debug)]
    struct TestDns {
        zones: Vec<TestDnsZone>,
    }

    impl Dns for TestDns {
        type DnsZone = TestDnsZone;
        fn list_zones(&self) -> Result<Vec<Self::DnsZone>, Error> {
            Ok(self.zones.clone())
        }
    }

    #[derive(Debug, Clone)]
    struct TestDnsZone {
        name: String,
    }

    impl TestDnsZone {
        fn new(name: &str) -> TestDnsZone {
            TestDnsZone {
                name: name.to_owned(),
            }
        }
    }

    impl DnsZone for TestDnsZone {
        fn id(&self) -> &str {
            &self.name
        }
        fn name(&self) -> &str {
            &self.name
        }
        fn bind(&self, _fqdn: &str, _target: DnsTarget) -> Result<(), Error> {
            unimplemented!();
        }
        fn unbind(&self, _fqdn: &str) -> Result<(), Error> {
            unimplemented!();
        }
    }
}
