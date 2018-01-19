use dns::Dns;
use dns::aws::dns_zone::AwsDnsZone;
use errors::*;
use rusoto_core::DefaultCredentialsProvider;
use rusoto_core::Region;
use rusoto_core::default_tls_client;
use rusoto_route53::Route53;
use rusoto_route53::Route53Client;
use std::rc::Rc;

mod dns_zone;

pub struct AwsDns {
    client: Rc<Route53>,
}

impl AwsDns {
    pub fn new() -> Result<AwsDns> {
        let provider = DefaultCredentialsProvider::new()
            .chain_err(|| "could not create credentials provider")?;
        let tls_client = default_tls_client().chain_err(|| "could not create TLS client")?;
        let region = Region::UsEast1;
        let route53 = Route53Client::new(tls_client, provider, region);
        Ok(AwsDns {
            client: Rc::new(route53),
        })
    }
}

impl Dns for AwsDns {
    type DnsZone = AwsDnsZone;

    fn list_zones(&self) -> Result<Vec<AwsDnsZone>> {
        AwsDnsZone::list(&self.client)
    }
}
