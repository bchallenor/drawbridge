extern crate clap;
#[macro_use]
extern crate failure;
extern crate futures;
extern crate hyper;
extern crate ipnet;
extern crate openssl_probe;
extern crate rusoto_core;
extern crate rusoto_ec2;
extern crate rusoto_route53;
extern crate tokio_core;

mod cli;
mod cloud;
mod dns;
mod iprules;

use cloud::aws::AwsCloud;
use dns::aws::AwsDns;
use failure::Error;
use std::env;
use std::process;

fn main() {
    match run() {
        Ok(()) => (),
        Err(error) => {
            // TODO(NLL)
            {
                if let Some(err) = error.downcast_ref::<clap::Error>() {
                    err.exit();
                }
            }
            eprintln!("{}", error);
            process::exit(1)
        }
    }
}

fn run() -> Result<(), Error> {
    // For e.g. Termux support on Android
    openssl_probe::init_ssl_cert_env_vars();

    let cmd = cli::parse_from_safe(env::args_os())?;

    let profile_opt = env::var("DRAWBRIDGE_PROFILE").ok();
    let profile = profile_opt.as_ref().map_or("default", String::as_ref);

    let tag_key = "Drawbridge";
    let tag_value = profile;
    println!("Filtering resources with tag: {}={}", tag_key, tag_value);

    let cloud = AwsCloud::new(tag_key, tag_value)?;
    let dns = AwsDns::new()?;

    cli::dispatch(cmd, &cloud, &dns)
}
