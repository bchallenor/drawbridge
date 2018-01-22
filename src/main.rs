extern crate clap;
#[macro_use]
extern crate error_chain;
extern crate futures;
extern crate hyper;
extern crate ipnet;
extern crate openssl_probe;
extern crate rusoto_core;
extern crate rusoto_ec2;
extern crate rusoto_route53;
extern crate tokio_core;

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
use futures::Future;
use futures::Stream;
use hyper::Client;
use hyper::StatusCode;
use ipnet::Ipv4Net;
use iprules::IpProtocol;
use std::net::Ipv4Addr;
use std::str;
use std::str::FromStr;
use tokio_core::reactor::Core;

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
        let ip_protocols = matches
            .values_of("protocol")
            .expect("required")
            .map(|x| {
                let y = match x {
                    "ssh" => "22/tcp",
                    "mosh" => "60000-61000/udp",
                    "http" => "80/tcp",
                    "https" => "443/tcp",
                    x => x,
                };
                if y != x {
                    println!("Substituted: {} -> {}", x, y);
                }
                IpProtocol::from_str(y).chain_err(|| format!("not a protocol: {}", y))
            })
            .collect::<Result<Vec<_>>>()?;

        let include_own_ip_addr = matches
            .values_of("source")
            .expect("required")
            .any(|x| x == "self");

        let mut ip_cidrs = matches
            .values_of("source")
            .expect("required")
            .filter(|&x| x != "self")
            .map(|x| {
                if x.contains('/') {
                    Ipv4Net::from_str(x).chain_err(|| format!("not an IP network: {}", x))
                } else {
                    Ipv4Addr::from_str(x)
                        .chain_err(|| format!("not an IP address: {}", x))
                        .map(|addr| Ipv4Net::new(addr, 32).expect("32 is OK"))
                }
            })
            .collect::<Result<Vec<_>>>()?;

        if include_own_ip_addr {
            let own_ip_addr = find_own_ip_addr()?;
            let own_ip_cidr = Ipv4Net::new(own_ip_addr, 32).expect("32 is OK");
            println!("Substituted: self -> {}", own_ip_cidr);
            ip_cidrs.push(own_ip_cidr);
        }

        let instance_type = matches.value_of("instance-type").map(InstanceType::new);

        Command::Start {
            ip_protocols,
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

fn find_own_ip_addr() -> Result<Ipv4Addr> {
    let mut core = Core::new().chain_err(|| "failed to create core reactor")?;
    let client = Client::new(&core.handle());
    let uri = "http://checkip.amazonaws.com/".parse().expect("valid URL");
    let (status, body) = core.run(
        client
            .get(uri)
            .and_then(|res| (futures::finished(res.status()), res.body().concat2())),
    ).chain_err(|| "failed to contact checkip service")?;
    let content = str::from_utf8(&*body).chain_err(|| "expected checkip to return UTF8")?;
    if status != StatusCode::Ok {
        bail!("checkip service returned {}: {}", status, content);
    }
    Ipv4Addr::from_str(content.trim_right())
        .chain_err(|| format!("expected checkip to return IP address: {}", content))
}
