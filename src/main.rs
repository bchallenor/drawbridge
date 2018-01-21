extern crate clap;
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

use clap::App;
use clap::AppSettings;
use clap::Arg;
use clap::SubCommand;
use cloud::InstanceType;
use cloud::aws::AwsCloud;
use dispatch::Command;
use dispatch::dispatch;
use dns::aws::AwsDns;
use errors::*;
use ipnet::Ipv4Net;
use iprules::IpService;
use std::str::FromStr;

quick_main!(run);

fn define_app<'a, 'b>() -> App<'a, 'b> {
    let open_command = SubCommand::with_name("open")
        .setting(AppSettings::DeriveDisplayOrder)
        .arg(
            Arg::with_name("protocol")
                .short("p")
                .long("protocol")
                .takes_value(true)
                .multiple(true)
                .required(true),
        )
        .arg(
            Arg::with_name("source")
                .short("s")
                .long("source")
                .takes_value(true)
                .multiple(true)
                .required(true),
        )
        .arg(
            Arg::with_name("instance-type")
                .short("t")
                .long("instance-type")
                .takes_value(true),
        );

    let close_command = SubCommand::with_name("close");

    App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .setting(AppSettings::GlobalVersion)
        .setting(AppSettings::VersionlessSubcommands)
        .setting(AppSettings::DeriveDisplayOrder)
        .subcommand(open_command)
        .subcommand(close_command)
}

fn run() -> Result<()> {
    // For e.g. Termux support on Android
    openssl_probe::init_ssl_cert_env_vars();

    let app = define_app();
    let matches = app.get_matches();

    let cmd = if let Some(matches) = matches.subcommand_matches("open") {
        let ip_services = matches
            .values_of("protocol")
            .expect("required")
            .map(|x| IpService::from_str(&x).chain_err(|| format!("not a protocol: {}", &x)))
            .collect::<Result<Vec<_>>>()?;

        let ip_cidrs = matches
            .values_of("source")
            .expect("required")
            .map(|x| Ipv4Net::from_str(&x).chain_err(|| format!("not a CIDR network: {}", &x)))
            .collect::<Result<Vec<_>>>()?;

        let instance_type = matches
            .value_of("instance-type")
            .map(|x| InstanceType(x.to_owned()));

        Command::Start {
            ip_services,
            ip_cidrs,
            instance_type,
        }
    } else if let Some(_matches) = matches.subcommand_matches("close") {
        Command::Stop
    } else {
        unreachable!()
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
