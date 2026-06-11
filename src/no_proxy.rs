use crate::pac::PacResolver;
use regex::Regex;
use std::collections::HashSet;

/// Extract hostnames/domains from the PAC script that evaluate to DIRECT.
pub fn extract_no_proxy(pac_script: &str, resolver: &PacResolver) -> Vec<String> {
    let mut no_proxy = HashSet::new();

    // Default local loopback/local link-local addresses
    let defaults = vec![
        "localhost".to_string(),
        "127.0.0.1".to_string(),
        "local".to_string(),
        "169.254/16".to_string(),
    ];
    for default in defaults {
        no_proxy.insert(default);
    }

    // Regex to extract single and double quoted string literals
    let re_string =
        Regex::new(r#"(?:"([^"\\]*(?:\\.[^"\\]*)*)"|'([^'\\]*(?:\\.[^'\\]*)*)')"#).unwrap();

    for cap in re_string.captures_iter(pac_script) {
        let literal = cap
            .get(1)
            .or_else(|| cap.get(2))
            .map(|m| m.as_str().trim())
            .unwrap_or("");

        if literal.is_empty() {
            continue;
        }

        // Ignore PAC keywords
        let lower = literal.to_lowercase();
        if lower == "direct"
            || lower == "proxy"
            || lower == "socks"
            || lower == "socks5"
            || lower == "socks4"
            || lower == "http"
            || lower == "https"
        {
            continue;
        }

        // Skip if it contains characters that are definitely not part of host/domain names
        if literal.contains(' ')
            || literal.contains(':')
            || literal.contains(';')
            || literal.contains('(')
            || literal.contains(')')
        {
            continue;
        }

        // Allow slashes only if it looks like an IP subnet (e.g. 10.0.0.0/8)
        if literal.contains('/') {
            let parts: Vec<&str> = literal.split('/').collect();
            if parts.len() != 2 {
                continue;
            }
            // First part should be an IP, second part a number
            if !parts[0].chars().all(|c| c.is_ascii_digit() || c == '.')
                || !parts[1].chars().all(|c| c.is_ascii_digit())
            {
                continue;
            }
        }

        // Clean and test candidates
        let (candidates_to_test, final_format) = if let Some(domain) = literal.strip_prefix("*.") {
            (vec![format!("test.{}", domain)], domain.to_string())
        } else if let Some(domain) = literal.strip_prefix('.') {
            (vec![format!("test.{}", domain)], domain.to_string())
        } else {
            (vec![literal.to_string()], literal.to_string())
        };

        if final_format.is_empty() || final_format.contains('*') {
            continue; // Ignore generic wildcards
        }

        // Test in the PAC resolver
        for test_host in candidates_to_test {
            let test_url = format!("http://{}", test_host);
            if let Some(res) = resolver.find_proxy_for_url(&test_url, &test_host) {
                if res.trim().to_uppercase() == "DIRECT" {
                    no_proxy.insert(final_format.clone());
                    break;
                }
            }
        }
    }

    let mut result: Vec<String> = no_proxy.into_iter().collect();
    result.sort();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_no_proxy() {
        let pac = r#"
            function FindProxyForURL(url, host) {
                if (dnsDomainIs(host, "my-company.com") || shExpMatch(host, "*.internal.net")) {
                    return "DIRECT";
                }
                return "PROXY proxy.com:8080";
            }
        "#;
        let resolver = PacResolver::new(pac.to_string()).unwrap();
        let list = extract_no_proxy(pac, &resolver);

        assert!(list.contains(&"my-company.com".to_string()));
        assert!(list.contains(&"internal.net".to_string()));
        assert!(list.contains(&"localhost".to_string()));
        assert!(list.contains(&"127.0.0.1".to_string()));
    }
}
