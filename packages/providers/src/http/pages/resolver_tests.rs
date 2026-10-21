use super::*;

fn addr(ip: &str) -> SocketAddr {
    SocketAddr::new(ip.parse().unwrap(), 0)
}

#[test]
fn public_addresses_pass() {
    let addrs = vec![addr("93.184.216.34"), addr("2606:2800:220:1::")];
    assert_eq!(public_only("cdn", addrs.clone()), Ok(addrs));
}

#[test]
fn any_private_address_refuses_the_name() {
    for bad in [
        "127.0.0.1",
        "10.1.2.3",
        "192.168.0.9",
        "169.254.169.254",
        "::1",
        "fd00::1",
    ] {
        let addrs = vec![addr("93.184.216.34"), addr(bad)];
        let err = public_only("evil.example", addrs).unwrap_err();
        assert!(err.to_string().contains(bad), "{err}");
    }
}

#[test]
fn no_address_is_refused() {
    assert!(public_only("void", vec![]).is_err());
}

/// Stands in for reqwest's and hyper's wrappers, which expose the cause as `source()`.
#[derive(Debug)]
struct Wrapped(Refusal);

impl fmt::Display for Wrapped {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("dns error")
    }
}

impl std::error::Error for Wrapped {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

#[test]
fn a_refusal_is_found_under_wrapping_errors() {
    let refusal = Refusal::NoAddress("void".into());
    assert_eq!(refusal_in(&Wrapped(refusal.clone())), Some(refusal));
    assert_eq!(refusal_in(&std::io::Error::other("plain")), None);
}

fn resolve(resolver: &PublicResolver, name: &str) -> Result<Vec<SocketAddr>, String> {
    let name = name.parse().unwrap();
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    rt.block_on(resolver.resolve(name))
        .map(|addrs| addrs.collect())
        .map_err(|e| e.to_string())
}

#[test]
fn only_the_proxy_name_may_resolve_to_a_private_address() {
    let resolver = PublicResolver {
        fixed: Some("127.0.0.1".parse().unwrap()),
        proxies: vec!["localhost".into()],
        timed_out: Default::default(),
    };
    assert_eq!(resolve(&resolver, "LocalHost"), Ok(vec![addr("127.0.0.1")]));
    let err = resolve(&resolver, "cdn.example").unwrap_err();
    assert!(err.contains("non-public address 127.0.0.1"), "{err}");
}
