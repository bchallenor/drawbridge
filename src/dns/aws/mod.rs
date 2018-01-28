use dns::Dns;
use dns::aws::dns_zone::AwsDnsZone;
use failure::Error;
use rusoto_core::Region;
use rusoto_route53::Route53;
use rusoto_route53::Route53Client;
use std::rc::Rc;

mod dns_zone;

pub struct AwsDns {
    client: Rc<Route53>,
}

impl AwsDns {
    pub fn new() -> Result<AwsDns, Error> {
        let region = Region::UsEast1;
        let route53 = Route53Client::simple(region);
        Ok(AwsDns {
            client: Rc::new(route53),
        })
    }
}

impl Dns for AwsDns {
    type DnsZone = AwsDnsZone;

    fn list_zones(&self) -> Result<Vec<AwsDnsZone>, Error> {
        AwsDnsZone::list(&self.client)
    }
}
