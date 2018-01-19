use errors::*;
use std::fmt;
use std::net::Ipv4Addr;
use std::str;

pub trait Dns {
    type DnsZone: DnsZone;

    fn list_zones(&self) -> Result<Vec<Self::DnsZone>>;

    fn find_authoritative_zone(&self, name: &str) -> Result<Self::DnsZone> {
        let parts: Vec<&str> = name.split_terminator('.').collect();
        let zones = self.list_zones()?;
        zones
            .into_iter()
            .filter(|zone| {
                let zone_parts: Vec<&str> = zone.name().split_terminator('.').collect();
                parts.ends_with(&zone_parts)
            })
            .max_by_key(|zone| zone.name().len())
            .ok_or_else(|| format!("could not find authoritative DNS zone for: {}", name).into())
    }
}

pub trait DnsZone: fmt::Debug {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn bind(&self, fqdn: &str, ip_addr: Ipv4Addr) -> Result<()>;
    fn unbind(&self, fqdn: &str) -> Result<()>;
}
