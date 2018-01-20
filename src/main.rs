#[macro_use]
extern crate error_chain;
extern crate ipnet;
extern crate openssl_probe;
extern crate rusoto_core;
extern crate rusoto_ec2;
extern crate rusoto_route53;

mod cloud;
mod dispatch;
mod dns;
mod errors;
mod iprules;

use cloud::InstanceType;
use cloud::aws::AwsCloud;
use dispatch::Command;
use dispatch::dispatch;
use dns::aws::AwsDns;
use errors::*;
use ipnet::Ipv4Net;
use std::env;
use std::str::FromStr;

quick_main!(run);

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
