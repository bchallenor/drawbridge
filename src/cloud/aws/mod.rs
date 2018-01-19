use cloud::Cloud;
use cloud::aws::firewall::AwsFirewall;
use cloud::aws::instance::AwsInstance;
use errors::*;
use rusoto_core::DefaultCredentialsProvider;
use rusoto_core::Region;
use rusoto_core::default_tls_client;
use rusoto_ec2::Ec2;
use rusoto_ec2::Ec2Client;
use rusoto_ec2::Filter;
use std::env;
use std::rc::Rc;

mod firewall;
mod instance;
mod tags;

pub struct AwsCloud {
    client: Rc<Ec2>,
    filter: Filter,
}

impl AwsCloud {
    pub fn new(tag_key: &str, tag_value: &str) -> Result<AwsCloud> {
        let provider = DefaultCredentialsProvider::new()
            .chain_err(|| "could not create credentials provider")?;
        let tls_client = default_tls_client().chain_err(|| "could not create TLS client")?;
        let region = AwsCloud::default_region()?;
        let ec2 = Ec2Client::new(tls_client, provider, region);
        Ok(AwsCloud {
            client: Rc::new(ec2),
            filter: Filter {
                name: Some(format!("tag:{}", tag_key)),
                values: Some(vec![tag_value.to_owned()]),
            },
        })
    }

    fn default_region() -> Result<Region> {
        let region_str =
            env::var("AWS_DEFAULT_REGION").chain_err(|| "env var AWS_DEFAULT_REGION is not set")?;
        let region = region_str
            .parse()
            .chain_err(|| format!("env var AWS_DEFAULT_REGION is invalid: {}", region_str))?;
        Ok(region)
    }
}

impl Cloud for AwsCloud {
    type Firewall = AwsFirewall;
    type Instance = AwsInstance;

    fn list_firewalls(&self) -> Result<Vec<AwsFirewall>> {
        AwsFirewall::list(&self.client, &self.filter)
    }

    fn list_instances(&self) -> Result<Vec<AwsInstance>> {
        AwsInstance::list(&self.client, &self.filter)
    }
}
