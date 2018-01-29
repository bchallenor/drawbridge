pub mod aws;
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
    fn bind(&self, fqdn: &str, ip_addr: Ipv4Addr) -> Result<(), Error>;
    fn unbind(&self, fqdn: &str) -> Result<(), Error>;
}
