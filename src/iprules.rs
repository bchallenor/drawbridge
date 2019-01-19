use ipnet::IpNet;
use std::fmt;
use std::result;
use std::str;

#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub struct IpPortRange(pub u16, pub u16);

impl fmt::Display for IpPortRange {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let &IpPortRange(ref from, ref to) = self;
        if from == to {
            write!(f, "{}", from)
        } else {
            write!(f, "{}-{}", from, to)
        }
    }
}

impl fmt::Debug for IpPortRange {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

#[derive(Fail, Debug, Copy, Clone, PartialEq, Eq)]
#[fail(display = "invalid IP port range")]
pub struct ParseIpPortRangeError(());

impl str::FromStr for IpPortRange {
    type Err = ParseIpPortRangeError;

    fn from_str(s: &str) -> result::Result<Self, Self::Err> {
        let parts = s
            .split('-')
            .map(|x| x.parse::<u16>().map_err(|_| ParseIpPortRangeError(())))
            .collect::<result::Result<Vec<_>, Self::Err>>()?;
        match parts.len() {
            1 => Ok(IpPortRange(parts[0], parts[0])),
            2 => Ok(IpPortRange(parts[0], parts[1])),
            _ => Err(ParseIpPortRangeError(())),
        }
    }
}

#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub enum IpProtocol {
    Tcp(IpPortRange),
    Udp(IpPortRange),
}

impl fmt::Display for IpProtocol {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &IpProtocol::Tcp(ref range) => write!(f, "{}/tcp", range),
            &IpProtocol::Udp(ref range) => write!(f, "{}/udp", range),
        }
    }
}

impl fmt::Debug for IpProtocol {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

#[derive(Fail, Debug, Copy, Clone, PartialEq, Eq)]
#[fail(display = "invalid IP protocol")]
pub struct ParseIpProtocolError(());

impl str::FromStr for IpProtocol {
    type Err = ParseIpProtocolError;

    fn from_str(s: &str) -> result::Result<Self, Self::Err> {
        let parts = s.split('/').collect::<Vec<_>>();
        if parts.len() == 2 {
            match parts[1] {
                "tcp" => {
                    let range = parts[0].parse().map_err(|_| ParseIpProtocolError(()))?;
                    Ok(IpProtocol::Tcp(range))
                }
                "udp" => {
                    let range = parts[0].parse().map_err(|_| ParseIpProtocolError(()))?;
                    Ok(IpProtocol::Udp(range))
                }
                _ => Err(ParseIpProtocolError(())),
            }
        } else {
            Err(ParseIpProtocolError(()))
        }
    }
}

#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub struct IpIngressRule(pub IpNet, pub IpProtocol);

impl fmt::Debug for IpIngressRule {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let &IpIngressRule(ref net, ref protocol) = self;
        write!(f, "{} -> {}", protocol, net)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_range_display_and_parse() {
        test_display_and_parse(IpPortRange(1, 1), "1");
        test_display_and_parse(IpPortRange(1, 10), "1-10");
        test_display_and_parse(IpPortRange(1, 65_535), "1-65535");
    }

    #[test]
    fn test_protocol_display_and_parse() {
        test_display_and_parse(IpProtocol::Tcp(IpPortRange(1, 1)), "1/tcp");
        test_display_and_parse(IpProtocol::Tcp(IpPortRange(1, 10)), "1-10/tcp");
        test_display_and_parse(IpProtocol::Tcp(IpPortRange(1, 65_535)), "1-65535/tcp");

        test_display_and_parse(IpProtocol::Udp(IpPortRange(1, 1)), "1/udp");
        test_display_and_parse(IpProtocol::Udp(IpPortRange(1, 10)), "1-10/udp");
        test_display_and_parse(IpProtocol::Udp(IpPortRange(1, 65_535)), "1-65535/udp");
    }

    fn test_display_and_parse<V>(v: V, s: &str)
    where
        V: fmt::Display + fmt::Debug + str::FromStr + PartialEq,
        <V as str::FromStr>::Err: fmt::Debug + PartialEq,
    {
        assert_eq!(v.to_string(), s);
        assert_eq!(s.parse(), Ok(v));
    }
}
