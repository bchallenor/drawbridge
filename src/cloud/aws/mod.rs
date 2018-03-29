use cloud::Cloud;
use cloud::aws::firewall::AwsFirewall;
use cloud::aws::instance::AwsInstance;
use failure::Error;
use failure::ResultExt;
use rusoto_core::Region;
use rusoto_ec2::Ec2;
use rusoto_ec2::Ec2Client;
use rusoto_ec2::Filter;
use std::env;
use std::rc::Rc;
use std::str::FromStr;

mod firewall;
mod instance;
mod tags;

pub struct AwsCloud {
    client: Rc<Ec2>,
}

impl AwsCloud {
    pub fn new() -> Result<AwsCloud, Error> {
        let region = AwsCloud::default_region()?;
        let ec2 = Ec2Client::simple(region);
        Ok(AwsCloud {
            client: Rc::new(ec2),
        })
    }

    fn default_region() -> Result<Region, Error> {
        let region_str =
            env::var("AWS_DEFAULT_REGION").context("env var AWS_DEFAULT_REGION is not set")?;
        let region = Region::from_str(&region_str)
            .with_context(|_e| format!("env var AWS_DEFAULT_REGION is invalid: {}", region_str))?;
        Ok(region)
    }
}

impl Cloud for AwsCloud {
    type Firewall = AwsFirewall;
    type Instance = AwsInstance;

    fn list_firewalls<'a, N, S>(&self, names: N) -> Result<Vec<AwsFirewall>, Error>
    where
        N: IntoIterator<Item = &'a S>,
        S: AsRef<str> + 'a,
    {
        AwsFirewall::list(&self.client, build_filter(names))
    }

    fn list_instances<'a, N, S>(&self, names: N) -> Result<Vec<AwsInstance>, Error>
    where
        N: IntoIterator<Item = &'a S>,
        S: AsRef<str> + 'a,
    {
        AwsInstance::list(&self.client, build_filter(names))
    }
}

fn build_filter<'a, N, S>(names: N) -> Filter
where
    N: IntoIterator<Item = &'a S>,
    S: AsRef<str> + 'a,
{
    Filter {
        name: Some("tag:Name".to_owned()),
        values: Some(names.into_iter().map(|x| x.as_ref().to_owned()).collect()),
    }
}
