pub const HOSTS_DIR: &str = "testdata/hosts-files";

#[test]
fn test_parse_ip_from_cidr() {
    use crate::parse_ip_from_cidr;

    let results = vec![
        ("192.168.12.1/16", "192.168.12.1"),
        ("10.0.0.0/8", "10.0.0.0"),
        ("fe80::abcd/128", "fe80::abcd"),
    ];

    for (cidr, ip) in results {
        assert_eq!(
            parse_ip_from_cidr(String::from(cidr)),
            String::from(ip),
            "{}",
            cidr
        );
    }
}

#[test]
fn test_domain_or_default() {
    use crate::{authority::DOMAIN_NAME, domain_or_default};
    use std::str::FromStr;
    use trust_dns_server::client::rr::Name;

    assert_eq!(
        domain_or_default(None).unwrap(),
        Name::from_str(DOMAIN_NAME).unwrap()
    );

    assert_eq!(
        domain_or_default(Some("zerotier")).unwrap(),
        Name::from_str("zerotier").unwrap()
    );

    assert_eq!(
        domain_or_default(Some("zerotier.tld")).unwrap(),
        Name::from_str("zerotier.tld").unwrap()
    );

    for bad in vec!["bad.", "~", "!", ".", ""] {
        assert!(domain_or_default(Some(bad)).is_err(), "{}", bad);
    }
}

#[test]
fn test_central_token() {
    use crate::central_token;

    assert!(central_token(None).is_none());
    std::env::set_var("ZEROTIER_CENTRAL_TOKEN", "abcdef");
    assert!(central_token(None).is_some());
    assert_eq!(central_token(None).unwrap(), "abcdef");

    let hosts = std::fs::read_to_string("/etc/hosts").unwrap();
    let token = central_token(Some("/etc/hosts"));
    assert!(token.is_some());
    assert_eq!(token.unwrap(), hosts.trim());
}

#[test]
#[should_panic]
fn test_central_token_panic() {
    use crate::central_token;
    central_token(Some("/nonexistent"));
}

#[test]
#[cfg(target_os = "linux")]
fn test_supervise_systemd_green() {
    let table = vec![
        (
            "basic",
            crate::supervise::Properties {
                binpath: String::from("zeronsd"),
                network: String::from("1234567891011121"),
                token: String::from("/proc/cpuinfo"),
                ..Default::default()
            },
        ),
        (
            "with-filled-in-properties",
            crate::supervise::Properties {
                binpath: String::from("zeronsd"),
                network: String::from("1234567891011121"),
                token: String::from("/proc/cpuinfo"),
                domain: Some(String::from("zerotier")),
                authtoken: Some(String::from("/var/lib/zerotier-one/authtoken.secret")),
                hosts_file: Some(String::from("/etc/hosts")),
            },
        ),
    ];

    let write = match std::env::var("WRITE_FIXTURES") {
        Ok(var) => var != "",
        Err(_) => false,
    };

    if write {
        eprintln!("Write mode: not testing, but updating unit files")
    }

    for (name, mut props) in table {
        let path = std::path::PathBuf::from(format!("testdata/supervise/systemd/{}.unit", name));

        if !write {
            let path = path.canonicalize();

            assert!(path.is_ok(), "{}", name);
            let expected = std::fs::read_to_string(path.unwrap());
            assert!(expected.is_ok(), "{}", name);
            let testing = props.systemd_template();
            assert!(testing.is_ok(), "{}", name);

            assert_eq!(testing.unwrap(), expected.unwrap(), "{}", name);
        } else {
            assert!(props.validate().is_ok(), "{}", name);

            let template = props.systemd_template();
            assert!(template.is_ok(), "{}", name);
            assert!(
                std::fs::write(path, props.systemd_template().unwrap()).is_ok(),
                "{}",
                name
            );
        }
    }
}

#[test]
#[cfg(target_os = "linux")]
fn test_supervise_systemd_red() {
    let table = vec![
        (
            "bad network",
            crate::supervise::Properties {
                binpath: String::from("zeronsd"),
                network: String::from("123456789101112"),
                token: String::from("/proc/cpuinfo"),
                ..Default::default()
            },
        ),
        (
            "bad token (no file)",
            crate::supervise::Properties {
                binpath: String::from("zeronsd"),
                network: String::from("1234567891011121"),
                token: String::from("~"),
                ..Default::default()
            },
        ),
        (
            "bad token (dir)",
            crate::supervise::Properties {
                binpath: String::from("zeronsd"),
                network: String::from("1234567891011121"),
                token: String::from("."),
                ..Default::default()
            },
        ),
        (
            "bad hosts (no file)",
            crate::supervise::Properties {
                binpath: String::from("zeronsd"),
                network: String::from("1234567891011121"),
                token: String::from("/proc/cpuinfo"),
                hosts_file: Some(String::from("~")),
                ..Default::default()
            },
        ),
        (
            "bad hosts (dir)",
            crate::supervise::Properties {
                binpath: String::from("zeronsd"),
                network: String::from("1234567891011121"),
                token: String::from("/proc/cpuinfo"),
                hosts_file: Some(String::from(".")),
                ..Default::default()
            },
        ),
        (
            "bad authtoken (no file)",
            crate::supervise::Properties {
                binpath: String::from("zeronsd"),
                network: String::from("1234567891011121"),
                token: String::from("/proc/cpuinfo"),
                authtoken: Some(String::from("~")),
                ..Default::default()
            },
        ),
        (
            "bad authtoken (dir)",
            crate::supervise::Properties {
                binpath: String::from("zeronsd"),
                network: String::from("1234567891011121"),
                token: String::from("/proc/cpuinfo"),
                authtoken: Some(String::from(".")),
                ..Default::default()
            },
        ),
        (
            "bad domain (empty string)",
            crate::supervise::Properties {
                binpath: String::from("zeronsd"),
                network: String::from("1234567891011121"),
                token: String::from("/proc/cpuinfo"),
                domain: Some(String::from("")),
                ..Default::default()
            },
        ),
        (
            "bad domain (invalid)",
            crate::supervise::Properties {
                binpath: String::from("zeronsd"),
                network: String::from("1234567891011121"),
                token: String::from("/proc/cpuinfo"),
                domain: Some(String::from("-")),
                ..Default::default()
            },
        ),
    ];

    for (name, mut props) in table {
        assert!(props.validate().is_err(), "{}", name);
    }
}

#[test]
fn test_parse_hosts() {
    use crate::hosts::parse_hosts;
    use std::net::IpAddr;
    use std::str::FromStr;
    use trust_dns_resolver::Name;

    let domain = &Name::from_str("zombocom").unwrap();

    for path in std::fs::read_dir(HOSTS_DIR)
        .unwrap()
        .into_iter()
        .map(|p| p.unwrap())
    {
        if path.metadata().unwrap().is_file() {
            let res = parse_hosts(Some(path.path().display().to_string()), domain.clone());
            assert!(res.is_ok(), "{}", path.path().display());

            let mut table = res.unwrap();

            assert_eq!(
                table
                    .remove(&IpAddr::from_str("127.0.0.1").unwrap())
                    .unwrap()
                    .first()
                    .unwrap(),
                &Name::from_str("localhost").unwrap().append_domain(domain),
                "{}",
                path.path().display(),
            );

            assert_eq!(
                table
                    .remove(&IpAddr::from_str("::1").unwrap())
                    .unwrap()
                    .first()
                    .unwrap(),
                &Name::from_str("localhost").unwrap().append_domain(domain),
                "{}",
                path.path().display(),
            );

            let mut accounted = vec!["islay.localdomain", "islay"]
                .into_iter()
                .map(|s| Name::from_str(s).unwrap().append_domain(domain));

            for name in table
                .remove(&IpAddr::from_str("127.0.1.1").unwrap())
                .unwrap()
            {
                assert!(accounted.any(|s| s.eq(&name)));
            }
        }
    }
}