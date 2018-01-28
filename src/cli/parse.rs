use clap::App;
use clap::AppSettings;
use clap::Arg;
use clap::SubCommand;
use cli::Command;
use cloud::InstanceType;
use failure::Error;
use futures;
use futures::Future;
use futures::Stream;
use hyper::Client;
use hyper::StatusCode;
use ipnet::IpNet;
use ipnet::Ipv4Net;
use ipnet::Ipv6Net;
use iprules::IpProtocol;
use std::ffi::OsString;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::str;
use std::str::FromStr;
use tokio_core::reactor::Core;

fn define_app<'a, 'b>() -> App<'a, 'b> {
    let open_command = SubCommand::with_name("open")
        .setting(AppSettings::DeriveDisplayOrder)
        .arg(
            Arg::with_name("protocol")
                .help(
                    "Protocol to allow through the firewall. Examples:\n\
                     * ssh\n\
                     * mosh\n\
                     * http\n\
                     * https\n\
                     * 22/tcp\n\
                     * 60000-61000/udp\n\
                     ",
                )
                .next_line_help(true)
                .short("p")
                .long("protocol")
                .takes_value(true)
                .multiple(true)
                .required(true),
        )
        .arg(
            Arg::with_name("source")
                .help(
                    "Source IP address (or CIDR network) to allow through the firewall.\n\
                     Examples:\n\
                     * self (alias for your IPv4 address, as indicated by checkip.amazonaws.com)\n\
                     * 192.0.2.1\n\
                     * 192.0.2.0/24\n\
                     ",
                )
                .next_line_help(true)
                .short("s")
                .long("source")
                .takes_value(true)
                .multiple(true)
                .required(true),
        )
        .arg(
            Arg::with_name("instance-type")
                .help(
                    "Desired instance type. Note that changing the instance type typically \
                     requires the instance to be stopped. Examples:\n\
                     * t2.nano\n\
                     * m3.medium\n\
                     * c5.large\n\
                     ",
                )
                .next_line_help(true)
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

pub fn parse_from_safe<I, T>(args: I) -> Result<Command, Error>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let app = define_app();
    let matches = app.get_matches_from_safe(args)?;

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
            .collect::<Result<Vec<_>, _>>()?;

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
                    IpNet::from_str(x).chain_err(|| format!("not an IP network: {}", x))
                } else {
                    IpAddr::from_str(x)
                        .chain_err(|| format!("not an IP address: {}", x))
                        .map(|addr| match addr {
                            IpAddr::V4(addr) => {
                                IpNet::V4(Ipv4Net::new(addr, 32).expect("32 is OK"))
                            }
                            IpAddr::V6(addr) => {
                                IpNet::V6(Ipv6Net::new(addr, 128).expect("128 is OK"))
                            }
                        })
                }
            })
            .collect::<Result<Vec<_>, _>>()?;

        if include_own_ip_addr {
            let own_ip_addr = find_own_ip_addr()?;
            let own_ip_cidr = IpNet::V4(Ipv4Net::new(own_ip_addr, 32).expect("32 is OK"));
            println!("Substituted: self -> {}", own_ip_cidr);
            ip_cidrs.push(own_ip_cidr);
        }

        let instance_type = matches.value_of("instance-type").map(InstanceType::new);

        Command::Open {
            ip_protocols,
            ip_cidrs,
            instance_type,
        }
    } else if let Some(_matches) = matches.subcommand_matches("close") {
        Command::Close
    } else {
        unreachable!()
    };

    Ok(cmd)
}

fn find_own_ip_addr() -> Result<Ipv4Addr, Error> {
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
    let ip_addr = Ipv4Addr::from_str(content.trim_right())
        .chain_err(|| format!("expected checkip to return IP address: {}", content))?;
    Ok(ip_addr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_open() {
        test_parse(
            &[
                "drawbridge",
                "open",
                "--protocol",
                "22/tcp",
                "--source",
                "1.1.1.1",
                "--source",
                "::ffff:1.1.1.1",
                "--instance-type",
                "m3.medium",
            ],
            Command::Open {
                ip_cidrs: vec![
                    "1.1.1.1/32".parse().unwrap(),
                    "::ffff:1.1.1.1/128".parse().unwrap(),
                ],
                ip_protocols: vec!["22/tcp".parse().unwrap()],
                instance_type: Some(InstanceType::new("m3.medium")),
            },
        ).unwrap();
    }

    #[test]
    fn test_parse_close() {
        test_parse(&["drawbridge", "close"], Command::Close).unwrap();
    }

    fn test_parse(args: &[&str], cmd: Command) -> Result<(), Error> {
        let actual_cmd = parse_from_safe(args)?;
        assert_eq!(cmd, actual_cmd);
        Ok(())
    }
}
