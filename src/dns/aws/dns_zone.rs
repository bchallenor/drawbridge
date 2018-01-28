use dns::DnsZone;
use failure::Error;
use rusoto_route53::Change;
use rusoto_route53::ChangeBatch;
use rusoto_route53::ChangeResourceRecordSetsRequest;
use rusoto_route53::ListHostedZonesRequest;
use rusoto_route53::ListResourceRecordSetsRequest;
use rusoto_route53::ResourceRecord;
use rusoto_route53::ResourceRecordSet;
use rusoto_route53::Route53;
use std::fmt;
use std::net::Ipv4Addr;
use std::rc::Rc;

pub struct AwsDnsZone {
    id: String,
    name: String,
    client: Rc<Route53>,
}

impl AwsDnsZone {
    pub(super) fn list(client: &Rc<Route53>) -> Result<Vec<AwsDnsZone>> {
        let req = ListHostedZonesRequest {
            ..Default::default()
        };
        let resp = client
            .list_hosted_zones(&req)
            .chain_err(|| format!("failed to list hosted zones: {:?}", req))?;
        let mut values = Vec::new();
        for hz in resp.hosted_zones {
            let value = AwsDnsZone {
                id: hz.id.trim_left_matches("/hostedzone/").to_owned(),
                name: hz.name,
                client: Rc::clone(client),
            };
            values.push(value);
        }
        Ok(values)
    }
}

impl fmt::Debug for AwsDnsZone {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} ({})", self.name, self.id)
    }
}

impl DnsZone for AwsDnsZone {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn bind(&self, fqdn: &str, ip_addr: Ipv4Addr) -> Result<()> {
        let desired = ResourceRecordSet {
            name: fqdn.to_owned(),
            resource_records: Some(vec![
                ResourceRecord {
                    value: ip_addr.to_string(),
                },
            ]),
            type_: "A".to_owned(),
            ttl: Some(60),
            ..Default::default()
        };
        self.change_record_set("UPSERT", desired)?;
        Ok(())
    }

    fn unbind(&self, fqdn: &str) -> Result<()> {
        if let Some(existing) = self.find_record_set(fqdn, "A")? {
            self.change_record_set("DELETE", existing)?;
        }
        Ok(())
    }
}

impl AwsDnsZone {
    fn find_record_set(&self, fqdn: &str, type_: &str) -> Result<Option<ResourceRecordSet>> {
        let req = ListResourceRecordSetsRequest {
            hosted_zone_id: self.id.clone(),
            start_record_name: Some(fqdn.to_owned()),
            start_record_type: Some(type_.to_owned()),
            max_items: Some("1".to_owned()), // ...String?
            ..Default::default()
        };
        let resp = self.client
            .list_resource_record_sets(&req)
            .chain_err(|| format!("failed to find existing DNS entry: {}", fqdn))?;
        Ok(resp.resource_record_sets.into_iter().next())
    }

    fn change_record_set(&self, action: &str, record_set: ResourceRecordSet) -> Result<()> {
        let fqdn = record_set.name.clone();
        let req = ChangeResourceRecordSetsRequest {
            hosted_zone_id: self.id.clone(),
            change_batch: ChangeBatch {
                comment: None,
                changes: vec![
                    Change {
                        action: action.to_owned(),
                        resource_record_set: record_set,
                    },
                ],
            },
        };
        self.client
            .change_resource_record_sets(&req)
            .chain_err(|| format!("failed to {} DNS entry: {}", action, fqdn))?;
        Ok(())
    }
}
