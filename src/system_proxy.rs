use std::fs;
use std::process::Command;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SystemProxySettings {
    pub pac_enabled: bool,
    pub pac_url: Option<String>,
    pub exceptions: Vec<String>,
    pub http_proxy: Option<String>,
    pub https_proxy: Option<String>,
    pub ftp_proxy: Option<String>,
}

#[derive(serde::Deserialize, Default, Debug)]
struct Config {
    pac_url: Option<String>,
    http_proxy: Option<String>,
    https_proxy: Option<String>,
    ftp_proxy: Option<String>,
    no_proxy: Option<String>,
}

/// Retrieve the proxy settings on Linux.
pub fn get_system_proxy_settings() -> SystemProxySettings {
    // 1. Check environment variable override
    if let Ok(pac_url) = std::env::var("PROXY_HELPER_PAC_URL") {
        if !pac_url.trim().is_empty() {
            return SystemProxySettings {
                pac_enabled: true,
                pac_url: Some(pac_url.trim().to_string()),
                ..SystemProxySettings::default()
            };
        }
    }

    // 2. Check user configuration file (~/.config/proxy_helper/config.toml)
    if let Ok(home) = std::env::var("HOME") {
        let config_dir =
            std::env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| format!("{}/.config", home));
        let config_path = format!("{}/proxy_helper/config.toml", config_dir);
        if let Some(settings) = parse_config_file(&config_path) {
            return settings;
        }
    }

    // 3. Check system-wide configuration file (/etc/proxy_helper/config.toml)
    if let Some(settings) = parse_config_file("/etc/proxy_helper/config.toml") {
        return settings;
    }

    // 4. Check GNOME desktop proxy settings
    if let Some(settings) = get_gnome_proxy_settings() {
        return settings;
    }

    // 5. Check KDE desktop proxy settings
    if let Some(settings) = get_kde_proxy_settings() {
        return settings;
    }

    SystemProxySettings::default()
}

fn parse_config_file(path: &str) -> Option<SystemProxySettings> {
    let content = fs::read_to_string(path).ok()?;
    let config: Config = toml::from_str(&content).ok()?;
    let mut settings = SystemProxySettings::default();

    if let Some(pac) = config.pac_url {
        settings.pac_enabled = true;
        settings.pac_url = Some(pac);
    } else {
        settings.pac_enabled = false;
        settings.http_proxy = config.http_proxy;
        settings.https_proxy = config.https_proxy;
        settings.ftp_proxy = config.ftp_proxy;
    }

    if let Some(no_proxy) = config.no_proxy {
        settings.exceptions = no_proxy
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }

    Some(settings)
}

fn trim_quotes(s: &str) -> &str {
    s.trim().trim_matches(|c| c == '\'' || c == '"')
}

fn parse_gnome_array(s: &str) -> Vec<String> {
    let trimmed = s.trim().trim_matches(|c| c == '[' || c == ']');
    if trimmed.is_empty() {
        return Vec::new();
    }
    trimmed
        .split(',')
        .map(|item| trim_quotes(item.trim()).to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

fn get_gnome_proxy(proto: &str) -> Option<String> {
    let host_out = Command::new("gsettings")
        .args(["get", &format!("org.gnome.system.proxy.{}", proto), "host"])
        .output()
        .ok()?;
    let port_out = Command::new("gsettings")
        .args(["get", &format!("org.gnome.system.proxy.{}", proto), "port"])
        .output()
        .ok()?;
    let host = trim_quotes(&String::from_utf8_lossy(&host_out.stdout)).to_string();
    let port = String::from_utf8_lossy(&port_out.stdout).trim().to_string();
    if !host.is_empty() && port != "0" {
        Some(format!("http://{}:{}", host, port))
    } else {
        None
    }
}

fn get_gnome_proxy_settings() -> Option<SystemProxySettings> {
    let mode_output = Command::new("gsettings")
        .args(["get", "org.gnome.system.proxy", "mode"])
        .output()
        .ok()?;

    if !mode_output.status.success() {
        return None;
    }

    let mode_str = String::from_utf8_lossy(&mode_output.stdout);
    let mode = trim_quotes(&mode_str);

    let mut settings = SystemProxySettings::default();

    if mode == "auto" {
        settings.pac_enabled = true;
        if let Ok(url_output) = Command::new("gsettings")
            .args(["get", "org.gnome.system.proxy", "autoconfig-url"])
            .output()
        {
            if url_output.status.success() {
                let url_str = String::from_utf8_lossy(&url_output.stdout);
                let url = trim_quotes(&url_str).to_string();
                if !url.is_empty() {
                    settings.pac_url = Some(url);
                }
            }
        }
    } else if mode == "manual" {
        settings.pac_enabled = false;
        settings.http_proxy = get_gnome_proxy("http");
        settings.https_proxy = get_gnome_proxy("https");
        settings.ftp_proxy = get_gnome_proxy("ftp");
    }

    if let Ok(ignore_out) = Command::new("gsettings")
        .args(["get", "org.gnome.system.proxy", "ignore-hosts"])
        .output()
    {
        if ignore_out.status.success() {
            let ignore_str = String::from_utf8_lossy(&ignore_out.stdout);
            settings.exceptions = parse_gnome_array(&ignore_str);
        }
    }

    Some(settings)
}

fn get_kde_proxy_settings() -> Option<SystemProxySettings> {
    let home = std::env::var("HOME").ok()?;
    let config_dir =
        std::env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| format!("{}/.config", home));

    let paths = [
        format!("{}/kioslaverc", config_dir),
        format!("{}/.kde4/share/config/kioslaverc", home),
        format!("{}/.kde/share/config/kioslaverc", home),
    ];

    for path in &paths {
        if let Ok(content) = fs::read_to_string(path) {
            if let Some(settings) = parse_kioslaverc(&content) {
                return Some(settings);
            }
        }
    }

    None
}

fn parse_kioslaverc(content: &str) -> Option<SystemProxySettings> {
    let mut settings = SystemProxySettings::default();
    let mut in_proxy_settings = false;
    let mut has_settings = false;

    let mut proxy_type = None;
    let mut pac_url = None;
    let mut http_proxy = None;
    let mut https_proxy = None;
    let mut ftp_proxy = None;
    let mut exceptions = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            in_proxy_settings = line == "[Proxy Settings]";
            continue;
        }
        if !in_proxy_settings {
            continue;
        }

        if let Some(pos) = line.find('=') {
            let key = line[..pos].trim();
            let val = line[pos + 1..].trim();
            has_settings = true;

            match key {
                "ProxyType" => proxy_type = val.parse::<i32>().ok(),
                "ProxyConfigScript" => pac_url = Some(val.to_string()),
                "httpProxy" => http_proxy = Some(val.to_string()),
                "httpsProxy" => https_proxy = Some(val.to_string()),
                "ftpProxy" => ftp_proxy = Some(val.to_string()),
                "NoProxyFor" => {
                    exceptions = val
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
                _ => {}
            }
        }
    }

    if !has_settings {
        return None;
    }

    match proxy_type {
        Some(2) => {
            settings.pac_enabled = true;
            settings.pac_url = pac_url;
        }
        Some(1) => {
            settings.pac_enabled = false;
            settings.http_proxy = http_proxy;
            settings.https_proxy = https_proxy;
            settings.ftp_proxy = ftp_proxy;
        }
        _ => {
            settings.pac_enabled = false;
        }
    }
    settings.exceptions = exceptions;

    Some(settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_kioslaverc_pac() {
        let sample = r#"
[Proxy Settings]
ProxyType=2
ProxyConfigScript=http://my-internal.com/proxy.pac
NoProxyFor=localhost, 127.0.0.1
"#;
        let settings = parse_kioslaverc(sample).unwrap();
        assert!(settings.pac_enabled);
        assert_eq!(
            settings.pac_url,
            Some("http://my-internal.com/proxy.pac".to_string())
        );
        assert_eq!(
            settings.exceptions,
            vec!["localhost".to_string(), "127.0.0.1".to_string()]
        );
    }

    #[test]
    fn test_parse_kioslaverc_manual() {
        let sample = r#"
[Proxy Settings]
ProxyType=1
httpProxy=http://10.0.0.1:8080
httpsProxy=http://10.0.0.1:8443
NoProxyFor=localhost, 127.0.0.1
"#;
        let settings = parse_kioslaverc(sample).unwrap();
        assert!(!settings.pac_enabled);
        assert_eq!(
            settings.http_proxy,
            Some("http://10.0.0.1:8080".to_string())
        );
        assert_eq!(
            settings.https_proxy,
            Some("http://10.0.0.1:8443".to_string())
        );
        assert_eq!(
            settings.exceptions,
            vec!["localhost".to_string(), "127.0.0.1".to_string()]
        );
    }

    #[test]
    fn test_parse_gnome_array() {
        assert_eq!(
            parse_gnome_array("['localhost', '127.0.0.1']"),
            vec!["localhost".to_string(), "127.0.0.1".to_string()]
        );
        assert_eq!(parse_gnome_array("[]"), Vec::<String>::new());
    }
}
