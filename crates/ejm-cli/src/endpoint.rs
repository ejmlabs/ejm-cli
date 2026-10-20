//! HTTPS-or-loopback check for URLs that can carry a credential.
//!
//! Provider base URLs use this today. A later registry flag can call the same
//! helper and choose its own error text. The proxy guard is separate: it only
//! applies to an `http` loopback URL that would otherwise be allowed.

use std::net::{Ipv4Addr, Ipv6Addr};

use url::{Host, Url};

/// Why a URL cannot carry a credential.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HttpsLoopbackError {
    /// The value is not an absolute `http` or `https` URL with a host.
    Invalid,
    /// The scheme is `http` and the host is not loopback.
    Cleartext,
}

/// Proxy variables that can carry a plain `http` request, plus the bypass list.
///
/// The snapshot stores which variable is set. It does not store the proxy URL.
#[derive(Clone, Debug, Default)]
pub(crate) struct ProxySnapshot {
    /// `HTTP_PROXY`, `http_proxy`, `ALL_PROXY`, or `all_proxy`, in that order.
    pub(crate) http_proxy_variable: Option<&'static str>,
    /// `NO_PROXY` when that variable is present, otherwise `no_proxy`.
    pub(crate) no_proxy: Option<String>,
}

impl ProxySnapshot {
    pub(crate) fn from_env() -> Self {
        const PROXY_VARIABLES: [&str; 4] = ["HTTP_PROXY", "http_proxy", "ALL_PROXY", "all_proxy"];
        Self {
            http_proxy_variable: PROXY_VARIABLES
                .into_iter()
                .find(|name| std::env::var(name).is_ok_and(|value| !value.trim().is_empty())),
            no_proxy: if std::env::var_os("NO_PROXY").is_some() {
                Some(std::env::var("NO_PROXY").unwrap_or_default())
            } else {
                std::env::var("no_proxy").ok()
            },
        }
    }

    #[cfg(test)]
    pub(crate) const fn none() -> Self {
        Self {
            http_proxy_variable: None,
            no_proxy: None,
        }
    }
}

/// Accept `https`, or `http` only for loopback.
///
/// Loopback is an exact ASCII case-insensitive `localhost`, an IPv4 address in
/// `127.0.0.0/8`, or IPv6 `::1`. Parsing does not resolve DNS or open a socket.
/// `0.0.0.0`, private LAN addresses, and every other host are cleartext.
/// Userinfo, a query, or a fragment is invalid for both schemes.
///
/// # Errors
///
/// Returns [`HttpsLoopbackError::Invalid`] when `value` does not parse, has no
/// host, does not use `http` or `https`, or contains userinfo, a query, or a
/// fragment. Returns [`HttpsLoopbackError::Cleartext`] for non-loopback `http`.
pub(crate) fn require_https_or_loopback(value: &str) -> Result<(), HttpsLoopbackError> {
    let url = Url::parse(value).map_err(|_| HttpsLoopbackError::Invalid)?;
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(HttpsLoopbackError::Invalid);
    }
    let Some(host) = url.host() else {
        return Err(HttpsLoopbackError::Invalid);
    };
    let loopback = host_is_loopback(&host);
    match url.scheme() {
        "https" => Ok(()),
        "http" if loopback => Ok(()),
        "http" => Err(HttpsLoopbackError::Cleartext),
        _ => Err(HttpsLoopbackError::Invalid),
    }
}

/// The proxy variable that would carry this `http` loopback URL, if any.
///
/// `NO_PROXY` / `no_proxy` covers the host when an entry is the exact host, the
/// same loopback IP, or `*` for a hostname. `*` does not cover an IP address:
/// the HTTP client on the current harness pin uses that same rule.
pub(crate) fn blocking_proxy_variable(
    value: &str,
    proxies: &ProxySnapshot,
) -> Option<&'static str> {
    let variable = proxies.http_proxy_variable?;
    let url = Url::parse(value).ok()?;
    if url.scheme() != "http" {
        return None;
    }
    let host = url.host()?;
    if !host_is_loopback(&host) {
        return None;
    }
    if proxies
        .no_proxy
        .as_deref()
        .is_some_and(|list| no_proxy_covers(list, &host))
    {
        return None;
    }
    Some(variable)
}

fn host_is_loopback(host: &Host<&str>) -> bool {
    match host {
        Host::Domain(name) => name.eq_ignore_ascii_case("localhost"),
        Host::Ipv4(address) => address.is_loopback(),
        Host::Ipv6(address) => address.is_loopback(),
    }
}

fn no_proxy_covers(list: &str, host: &Host<&str>) -> bool {
    list.split(',')
        .any(|entry| entry_covers(entry.trim(), host))
}

fn entry_covers(entry: &str, host: &Host<&str>) -> bool {
    if entry.is_empty() {
        return false;
    }
    if entry == "*" {
        return matches!(host, Host::Domain(_));
    }
    match host {
        Host::Domain(name) => entry.eq_ignore_ascii_case(name),
        Host::Ipv4(address) => entry.parse::<Ipv4Addr>().ok() == Some(*address),
        Host::Ipv6(address) => {
            let bare = entry
                .strip_prefix('[')
                .and_then(|value| value.strip_suffix(']'))
                .unwrap_or(entry);
            bare.parse::<Ipv6Addr>().ok() == Some(*address)
        }
    }
}

/// Host, port, and path of `value`, without userinfo, query, fragment, or controls.
pub(crate) fn display_without_userinfo(value: &str) -> String {
    let Ok(url) = Url::parse(value) else {
        return String::new();
    };
    let Some(host) = url.host_str() else {
        return String::new();
    };
    let port = url.port().map_or(String::new(), |port| format!(":{port}"));
    one_line(&format!("{}://{host}{port}{}", url.scheme(), url.path()))
}

fn one_line(value: &str) -> String {
    value
        .chars()
        .filter(|character| {
            !character.is_control() && !matches!(character, '\u{2028}' | '\u{2029}')
        })
        .collect()
}

#[cfg(test)]
#[expect(
    clippy::unwrap_used,
    reason = "URL fixtures in this module must parse or fail the test."
)]
mod tests {
    use super::*;

    #[test]
    fn url_2_5_normalizes_loopback_spellings() {
        for value in [
            "http://127.1/v1",
            "http://0x7f.0.0.1/v1",
            "http://2130706433/v1",
            "http://0177.0.0.1/v1",
            "http://127.0.0.1./v1",
        ] {
            let host = Url::parse(value).unwrap().host().unwrap().to_string();
            assert_eq!(host, "127.0.0.1", "{value} -> {host}");
            assert_eq!(require_https_or_loopback(value), Ok(()), "{value}");
        }
        assert_eq!(
            Url::parse("http://0/v1")
                .unwrap()
                .host()
                .unwrap()
                .to_string(),
            "0.0.0.0"
        );
        assert_eq!(
            require_https_or_loopback("http://0/v1"),
            Err(HttpsLoopbackError::Cleartext)
        );
    }

    #[test]
    fn host_classes_match_the_loopback_set() {
        for value in [
            "http://127.255.255.255/v1",
            "http://[0:0:0:0:0:0:0:1]/v1",
            "http://[::0001]/v1",
            "http://[0000:0000::0001]:8080/v1",
            "HTTP://LOCALHOST/v1",
            "http://localhost\\@evil.invalid/v1",
            "http://ｌｏｃａｌｈｏｓｔ/v1",
            "http://%6c%6f%63%61%6c%68%6f%73%74/v1",
        ] {
            assert_eq!(require_https_or_loopback(value), Ok(()), "{value}");
        }
        for value in [
            "http://[fe80::1]/v1",
            "http://[fc00::1]/v1",
            "http://[fd00::1]/v1",
            "http://169.254.169.254/v1",
            "http://192.168.0.1/v1",
            "http://172.31.255.255/v1",
            "http://foo.localhost/v1",
            "http://localhost./v1",
            "http://evil-localhost/v1",
            "http://evil.invalid\\@localhost/v1",
        ] {
            assert_eq!(
                require_https_or_loopback(value),
                Err(HttpsLoopbackError::Cleartext),
                "{value}"
            );
        }
        assert_eq!(
            require_https_or_loopback("http://::1/v1"),
            Err(HttpsLoopbackError::Invalid)
        );
    }

    #[test]
    fn userinfo_query_and_fragment_are_invalid() {
        for value in [
            "https://user:secret@api.example.invalid/v1",
            "http://user:secret@localhost/v1",
            "http://evil.invalid@localhost/v1",
            "http://localhost@evil.invalid/v1",
            "https://api.example.invalid/v1?key=sk-secret",
            "https://api.example.invalid/v1#fragment",
        ] {
            assert_eq!(
                require_https_or_loopback(value),
                Err(HttpsLoopbackError::Invalid),
                "{value}"
            );
        }
    }

    #[test]
    fn display_drops_userinfo_and_keeps_ipv6_brackets() {
        let shown = display_without_userinfo("http://user:secret@example.invalid/v1");
        assert_eq!(shown, "http://example.invalid/v1");
        assert!(!shown.contains("secret"));
        assert_eq!(
            display_without_userinfo("http://[fe80::1]:8080/v1"),
            "http://[fe80::1]:8080/v1"
        );
        let injected = display_without_userinfo("http://localhost/v1\nForged: line");
        assert!(!injected.contains('\n'));
        assert_eq!(injected.lines().count(), 1);
    }

    #[test]
    fn proxy_guard_uses_exact_host_loopback_ip_and_star() {
        let blocked = |url, variable: &'static str, no_proxy: Option<&str>| {
            blocking_proxy_variable(
                url,
                &ProxySnapshot {
                    http_proxy_variable: Some(variable),
                    no_proxy: no_proxy.map(str::to_owned),
                },
            )
        };
        assert_eq!(
            blocked("http://127.0.0.1:9/v1", "HTTP_PROXY", None),
            Some("HTTP_PROXY")
        );
        assert_eq!(
            blocked("http://127.0.0.1/v1", "http_proxy", None),
            Some("http_proxy")
        );
        assert_eq!(
            blocked("http://localhost/v1", "ALL_PROXY", None),
            Some("ALL_PROXY")
        );
        assert_eq!(
            blocked("http://[::1]/v1", "all_proxy", None),
            Some("all_proxy")
        );
        assert_eq!(
            blocked("http://127.0.0.1/v1", "HTTP_PROXY", None),
            Some("HTTP_PROXY")
        );
        assert_eq!(
            blocking_proxy_variable("http://127.0.0.1/v1", &ProxySnapshot::none()),
            None
        );
        assert_eq!(
            blocked("http://127.0.0.1/v1", "HTTP_PROXY", Some("127.0.0.1")),
            None
        );
        assert_eq!(
            blocked("http://127.0.0.2/v1", "HTTP_PROXY", Some("127.0.0.1")),
            Some("HTTP_PROXY")
        );
        assert_eq!(blocked("http://[::1]/v1", "HTTP_PROXY", Some("::1")), None);
        assert_eq!(
            blocked("http://[0:0:0:0:0:0:0:1]/v1", "HTTP_PROXY", Some("[::1]")),
            None
        );
        assert_eq!(
            blocked("http://localhost/v1", "HTTP_PROXY", Some("localhost")),
            None
        );
        assert_eq!(
            blocked("http://LOCALHOST/v1", "HTTP_PROXY", Some("localhost")),
            None
        );
        assert_eq!(
            blocked("http://127.0.0.1/v1", "HTTP_PROXY", Some("localhost")),
            Some("HTTP_PROXY")
        );
        assert_eq!(blocked("http://localhost/v1", "ALL_PROXY", Some("*")), None);
        assert_eq!(
            blocked("http://127.0.0.1/v1", "HTTP_PROXY", Some("*")),
            Some("HTTP_PROXY")
        );
        assert_eq!(blocked("https://127.0.0.1/v1", "HTTP_PROXY", None), None);
        assert_eq!(
            blocked("http://api.example.invalid/v1", "HTTP_PROXY", None),
            None
        );
    }
}
