#[macro_use]
extern crate failure;

mod cli;
mod cloud;
mod dns;
mod iprules;

use crate::cloud::aws::AwsCloud;
use crate::dns::aws::AwsDns;
use clap;
use failure::Error;
use openssl_probe;
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

    let cloud = AwsCloud::new()?;
    let dns = AwsDns::new()?;

    cli::dispatch(cmd, &cloud, &dns)
}
