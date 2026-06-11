mod no_proxy;
mod pac;
mod system_proxy;

use clap::Parser;
use std::collections::HashSet;
use std::net::IpAddr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShellStyle {
    BourneShell,
    CShell,
    Fish,
    PowerShell,
}

#[derive(Parser, Debug)]
#[command(
    name = "proxy_helper",
    version,
    about = "Helper for constructing proxy environment variables from Linux system settings (GNOME, KDE, config file) or a Proxy Auto-Configuration (PAC) file"
)]
struct Cli {
    /// Generate C-shell commands on stdout
    #[arg(short = 'c', long = "csh")]
    csh: bool,

    /// Generate Bourne shell commands on stdout
    #[arg(short = 's', long = "sh")]
    sh: bool,

    /// Generate Fish shell commands on stdout
    #[arg(short = 'f', long = "fish")]
    fish: bool,

    /// Generate PowerShell commands on stdout
    #[arg(short = 'w', long = "powershell")]
    powershell: bool,

    /// Path or URL to the Proxy Auto-Configuration (PAC) file (overrides system settings)
    #[arg(short = 'p', long = "pac")]
    pac: Option<String>,

    /// Target host or URL to use for all protocols (default: www.google.com)
    #[arg(short = 'u', long = "url", default_value = "www.google.com")]
    url: String,

    /// Specific target host or URL for HTTP proxy resolution
    #[arg(long = "http-url")]
    http_url: Option<String>,

    /// Specific target host or URL for HTTPS proxy resolution
    #[arg(long = "https-url")]
    https_url: Option<String>,

    /// Specific target host or URL for FTP proxy resolution
    #[arg(long = "ftp-url")]
    ftp_url: Option<String>,
}

fn main() {
    let cli = Cli::parse();

    // 1. Determine shell style
    let shell_style = if cli.csh {
        ShellStyle::CShell
    } else if cli.sh {
        ShellStyle::BourneShell
    } else if cli.fish {
        ShellStyle::Fish
    } else if cli.powershell {
        ShellStyle::PowerShell
    } else {
        // Fallback to checking the SHELL environment variable
        match std::env::var("SHELL") {
            Ok(shell) => {
                if shell.ends_with("csh") {
                    ShellStyle::CShell
                } else if shell.ends_with("fish") {
                    ShellStyle::Fish
                } else if shell.ends_with("pwsh") || shell.ends_with("powershell") {
                    ShellStyle::PowerShell
                } else {
                    ShellStyle::BourneShell
                }
            }
            Err(_) => ShellStyle::BourneShell,
        }
    };

    // 2. Fetch system proxy settings (GNOME, KDE, config file)
    let system_settings = system_proxy::get_system_proxy_settings();

    // 3. Determine if we are in PAC mode or Manual mode
    let pac_url = cli.pac.clone().or_else(|| {
        if system_settings.pac_enabled {
            system_settings.pac_url.clone()
        } else {
            None
        }
    });

    let mut http_proxy = None;
    let mut https_proxy = None;
    let mut ftp_proxy = None;
    let mut no_proxy_list = HashSet::new();

    if let Some(url_str) = pac_url {
        // PAC Mode
        match pac::fetch_pac(&url_str) {
            Ok(pac_script) => {
                match pac::PacResolver::new(pac_script.clone()) {
                    Ok(resolver) => {
                        // Resolve proxy for each protocol
                        let http_target = cli.http_url.as_deref().unwrap_or(&cli.url);
                        let (http_url_str, http_host) =
                            get_target_url_and_host(http_target, "http");
                        if let Some(res) = resolver.find_proxy_for_url(&http_url_str, &http_host) {
                            http_proxy = pac::parse_proxy_result(&res);
                        }

                        let https_target = cli.https_url.as_deref().unwrap_or(&cli.url);
                        let (https_url_str, https_host) =
                            get_target_url_and_host(https_target, "https");
                        if let Some(res) = resolver.find_proxy_for_url(&https_url_str, &https_host)
                        {
                            https_proxy = pac::parse_proxy_result(&res);
                        }

                        let ftp_target = cli.ftp_url.as_deref().unwrap_or(&cli.url);
                        let (ftp_url_str, ftp_host) = get_target_url_and_host(ftp_target, "ftp");
                        if let Some(res) = resolver.find_proxy_for_url(&ftp_url_str, &ftp_host) {
                            ftp_proxy = pac::parse_proxy_result(&res);
                        }

                        // Extract no_proxy candidates from PAC
                        let pac_no_proxy = no_proxy::extract_no_proxy(&pac_script, &resolver);
                        for item in pac_no_proxy {
                            if let Some(cleaned) = clean_exception(&item) {
                                no_proxy_list.insert(cleaned);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("[proxy_helper] Error compiling PAC script: {:?}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!(
                    "[proxy_helper] Error fetching PAC script ({}): {:?}",
                    url_str, e
                );
            }
        }
    } else {
        // Manual/Fallback Mode
        http_proxy = system_settings.http_proxy;
        https_proxy = system_settings.https_proxy;
        ftp_proxy = system_settings.ftp_proxy;
    }

    // Merge system exceptions into no_proxy list
    for exc in &system_settings.exceptions {
        if let Some(cleaned) = clean_exception(exc) {
            no_proxy_list.insert(cleaned);
        }
    }

    // Format no_proxy as comma-separated string
    let no_proxy = if !no_proxy_list.is_empty() {
        let mut sorted: Vec<String> = no_proxy_list.into_iter().collect();
        sorted.sort();
        Some(sorted.joined(","))
    } else {
        None
    };

    // 4. Output the environment variables
    let mut outputs = Vec::new();

    let format_env = |name: &str, val: &str| -> String {
        match shell_style {
            ShellStyle::BourneShell => format!("{}=\"{}\"; export {};", name, val, name),
            ShellStyle::CShell => format!("setenv {} \"{}\";", name, val),
            ShellStyle::Fish => format!("set -gx {} \"{}\";", name, val),
            ShellStyle::PowerShell => format!("$env:{} = \"{}\";", name, val),
        }
    };

    if let Some(val) = http_proxy {
        outputs.push(format_env("http_proxy", &val));
    }
    if let Some(val) = https_proxy {
        outputs.push(format_env("https_proxy", &val));
    }
    if let Some(val) = ftp_proxy {
        outputs.push(format_env("ftp_proxy", &val));
    }
    if let Some(val) = no_proxy {
        outputs.push(format_env("no_proxy", &val));
    }

    if !outputs.is_empty() {
        println!("{}", outputs.join(" "));
    }
}

/// Helper to parse a target input into (url, host).
fn get_target_url_and_host(target: &str, default_scheme: &str) -> (String, String) {
    if target.contains("://") {
        if let Ok(parsed) = url::Url::parse(target) {
            let host = parsed.host_str().unwrap_or("").to_string();
            let port = parsed.port().map(|p| format!(":{}", p)).unwrap_or_default();
            return (target.to_string(), format!("{}{}", host, port));
        }
    }

    let host = target.to_string();
    let url = format!("{}://{}", default_scheme, target);
    (url, host)
}

/// Helper to clean wildcard prefixes and validate IP/CIDR formats from bypass exception rules.
fn clean_exception(exc: &str) -> Option<String> {
    let exc = exc.trim();
    if exc.is_empty() {
        return None;
    }

    // 1. Handle wildcard domain patterns
    if let Some(domain) = exc.strip_prefix("*.") {
        return Some(domain.to_string());
    }
    if let Some(domain) = exc.strip_prefix('.') {
        return Some(domain.to_string());
    }

    // 2. Strip brackets for IPv6 compatibility
    let cleaned = exc.replace(['[', ']'], "");

    // 3. Handle IP address and CIDR subnet validations
    if cleaned.contains('/') {
        let parts: Vec<&str> = cleaned.split('/').collect();
        if parts.len() == 2 {
            let ip_part = parts[0];
            let mask_part = parts[1];
            if let (Ok(ip), Ok(mask)) = (ip_part.parse::<IpAddr>(), mask_part.parse::<u8>()) {
                match ip {
                    IpAddr::V4(_) if mask <= 32 => return Some(cleaned),
                    IpAddr::V6(_) if mask <= 128 => return Some(cleaned),
                    _ => {}
                }
            }
        }
    } else {
        if cleaned.parse::<IpAddr>().is_ok() {
            return Some(cleaned);
        }
    }

    // 4. Fallback: If it's a domain-like string (must not look like an invalid IP/CIDR containing colons/slashes)
    if cleaned.contains('/') || cleaned.contains(':') {
        return None;
    }

    Some(cleaned)
}

// Custom trait to join vector of strings (since std only has slice join)
trait Joinable {
    fn joined(&self, sep: &str) -> String;
}

impl Joinable for Vec<String> {
    fn joined(&self, sep: &str) -> String {
        self.join(sep)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_target_url_and_host() {
        assert_eq!(
            get_target_url_and_host("www.google.com", "http"),
            (
                "http://www.google.com".to_string(),
                "www.google.com".to_string()
            )
        );
        assert_eq!(
            get_target_url_and_host("https://secure.google.com:8443/search?q=rust", "https"),
            (
                "https://secure.google.com:8443/search?q=rust".to_string(),
                "secure.google.com:8443".to_string()
            )
        );
    }

    #[test]
    fn test_clean_exception_advanced() {
        assert_eq!(clean_exception("*.local"), Some("local".to_string()));
        assert_eq!(
            clean_exception(".company.com"),
            Some("company.com".to_string())
        );
        assert_eq!(clean_exception("127.0.0.1"), Some("127.0.0.1".to_string()));
        assert_eq!(
            clean_exception("192.168.1.0/24"),
            Some("192.168.1.0/24".to_string())
        );
        assert_eq!(clean_exception("fe80::1"), Some("fe80::1".to_string()));
        assert_eq!(clean_exception("[fe80::2]"), Some("fe80::2".to_string()));
        assert_eq!(clean_exception("fe80::/64"), Some("fe80::/64".to_string()));
        assert_eq!(
            clean_exception("[fe80::]/64"),
            Some("fe80::/64".to_string())
        );
        assert_eq!(
            clean_exception("example.com"),
            Some("example.com".to_string())
        );

        // Invalid cases
        assert_eq!(clean_exception("invalid_ip/999"), None);
        assert_eq!(clean_exception("1.1.1.1/33"), None);
        assert_eq!(clean_exception("fe80::1/129"), None);
        assert_eq!(clean_exception(""), None);
    }
}
