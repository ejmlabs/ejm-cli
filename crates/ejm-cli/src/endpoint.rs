//! HTTPS-or-loopback check for URLs that can carry a credential.
//!
//! Provider base URLs use this today. A later registry flag can call the same
//! helper and choose its own error text.

use url::{Host, Url};

/// Why a URL cannot carry a credential.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HttpsLoopbackError {
    /// The value is not an absolute `http` or `https` URL with a host.
    Invalid,
    /// The scheme is `http` and the host is not loopback.
    Cleartext,
}

/// Accept `https`, or `http` only for loopback.
///
/// Loopback is an exact ASCII case-insensitive `localhost`, an IPv4 address in
/// `127.0.0.0/8`, or IPv6 `::1`. Parsing does not resolve DNS or open a socket.
/// `0.0.0.0`, private LAN addresses, and every other host are cleartext.
///
/// # Errors
///
/// Returns [`HttpsLoopbackError::Invalid`] when `value` does not parse, has no
/// host, or does not use `http` or `https`. Returns
/// [`HttpsLoopbackError::Cleartext`] for non-loopback `http`.
pub(crate) fn require_https_or_loopback(value: &str) -> Result<(), HttpsLoopbackError> {
    let url = Url::parse(value).map_err(|_| HttpsLoopbackError::Invalid)?;
    let Some(host) = url.host() else {
        return Err(HttpsLoopbackError::Invalid);
    };
    let loopback = match host {
        Host::Domain(name) => name.eq_ignore_ascii_case("localhost"),
        Host::Ipv4(address) => address.is_loopback(),
        Host::Ipv6(address) => address.is_loopback(),
    };
    match url.scheme() {
        "https" => Ok(()),
        "http" if loopback => Ok(()),
        "http" => Err(HttpsLoopbackError::Cleartext),
        _ => Err(HttpsLoopbackError::Invalid),
    }
}

/// Host, port, and path of `value`, without userinfo, query, or fragment.
pub(crate) fn display_without_userinfo(value: &str) -> String {
    let Ok(url) = Url::parse(value) else {
        return String::new();
    };
    let Some(host) = url.host_str() else {
        return String::new();
    };
    let host = if host.contains(':') {
        format!("[{host}]")
    } else {
        host.to_owned()
    };
    let port = url.port().map_or(String::new(), |port| format!(":{port}"));
    format!("{}://{host}{port}{}", url.scheme(), url.path())
}

#[cfg(test)]
#[expect(
    clippy::unwrap_used,
    reason = "URL fixtures in this module must parse or fail the test."
)]
mod tests {
    use super::*;

    #[test]
    fn shortened_ipv4_follows_the_parsed_loopback_bit() {
        let value = "http://127.1/v1";
        let parsed = Url::parse(value).unwrap();
        let loopback = match parsed.host() {
            Some(Host::Ipv4(address)) => address.is_loopback(),
            Some(Host::Ipv6(address)) => address.is_loopback(),
            Some(Host::Domain(name)) => name.eq_ignore_ascii_case("localhost"),
            None => false,
        };
        assert_eq!(require_https_or_loopback(value).is_ok(), loopback);
    }

    #[test]
    fn display_drops_userinfo() {
        let shown = display_without_userinfo("http://user:secret@example.com/v1");
        assert_eq!(shown, "http://example.com/v1");
        assert!(!shown.contains("secret"));
    }
}
