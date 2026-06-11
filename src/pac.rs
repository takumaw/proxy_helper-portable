use rquickjs::{Context, Function, Runtime};
use url::Url;

const PAC_HELPERS: &str = r#"
function isPlainHostName(host) {
    return host.indexOf('.') === -1;
}

function dnsDomainIs(host, domain) {
    return host === domain || host.endsWith('.' + domain);
}

function localHostOrDomainIs(host, hostdom) {
    return host === hostdom || (host.indexOf('.') === -1 && hostdom.startsWith(host));
}

function dnsDomainLevels(host) {
    return (host.match(/\./g) || []).length;
}

function shExpMatch(str, shexp) {
    var re = new RegExp('^' + shexp.replace(/[\-\[\]\/\{\}\(\)\+\.\\\^\$\|]/g, "\\$&").replace(/\*/g, '.*').replace(/\?/g, '.') + '$', 'i');
    return re.test(str);
}

function isInNet(ip_addr, pattern, mask) {
    if (!/^\d+\.\d+\.\d+\.\d+$/.test(ip_addr)) {
        ip_addr = dnsResolve(ip_addr);
        if (!ip_addr) return false;
    }
    function ipToNum(ip) {
        var parts = ip.split('.');
        return ((parts[0] << 24) | (parts[1] << 16) | (parts[2] << 8) | parts[3]) >>> 0;
    }
    var ipNum = ipToNum(ip_addr);
    var patternNum = ipToNum(pattern);
    var maskNum = ipToNum(mask);
    return (ipNum & maskNum) === (patternNum & maskNum);
}

function isResolvable(host) {
    var ip = dnsResolve(host);
    return ip !== null && ip !== undefined && ip !== "";
}

function weekdayRange(wd1, wd2, gmt) {
    var days = ["SUN", "MON", "TUE", "WED", "THU", "FRI", "SAT"];
    var date = new Date();
    var day = (gmt === "GMT") ? date.getUTCDay() : date.getDay();
    var current = days[day];
    if (!wd2) return current === wd1;
    var idx1 = days.indexOf(wd1);
    var idx2 = days.indexOf(wd2);
    var curIdx = day;
    if (idx1 <= idx2) {
        return curIdx >= idx1 && curIdx <= idx2;
    } else {
        return curIdx >= idx1 || curIdx <= idx2;
    }
}

function dateRange() {
    return true;
}

function timeRange() {
    return true;
}
"#;

// Local IP discovery
fn my_ip_address() -> String {
    if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") {
        if socket.connect("8.8.8.8:80").is_ok() {
            if let Ok(local_addr) = socket.local_addr() {
                return local_addr.ip().to_string();
            }
        }
    }
    "127.0.0.1".to_string()
}

// DNS resolution
fn dns_resolve(host: String) -> Option<String> {
    use std::net::ToSocketAddrs;
    let host_port = format!("{}:80", host);
    if let Ok(addrs) = host_port.to_socket_addrs() {
        for addr in addrs {
            if let std::net::SocketAddr::V4(v4) = addr {
                return Some(v4.ip().to_string());
            }
        }
    }
    None
}

// PAC Alert
fn alert(msg: String) {
    eprintln!("[PAC Alert] {}", msg);
}

pub struct PacResolver {
    _runtime: Runtime,
    context: Context,
}

impl PacResolver {
    /// Initialize the PAC resolver with the script content.
    pub fn new(pac_script: String) -> Result<Self, rquickjs::Error> {
        let runtime = Runtime::new()?;
        let context = Context::full(&runtime)?;

        context.with(|ctx| {
            let globals = ctx.globals();
            globals.set("myIpAddress", Function::new(ctx.clone(), my_ip_address)?)?;
            globals.set("dnsResolve", Function::new(ctx.clone(), dns_resolve)?)?;
            globals.set("alert", Function::new(ctx.clone(), alert)?)?;

            ctx.eval::<(), _>(PAC_HELPERS)?;
            ctx.eval::<(), _>(pac_script.as_str())?;
            Ok::<(), rquickjs::Error>(())
        })?;

        Ok(Self {
            _runtime: runtime,
            context,
        })
    }

    /// Run FindProxyForURL on the context.
    pub fn find_proxy_for_url(&self, url: &str, host: &str) -> Option<String> {
        self.context.with(|ctx| {
            let find_proxy: Function = match ctx.globals().get("FindProxyForURL") {
                Ok(f) => f,
                _ => return None,
            };
            let result: String = match find_proxy.call((url, host)) {
                Ok(r) => r,
                _ => return None,
            };
            Some(result)
        })
    }
}

/// Fetch the PAC file content. Supports http, https, file, and local paths.
pub fn fetch_pac(path_or_url: &str) -> Result<String, Box<dyn std::error::Error>> {
    if path_or_url.starts_with("http://") || path_or_url.starts_with("https://") {
        let response = ureq::get(path_or_url).call()?;
        let content = response.into_string()?;
        Ok(content)
    } else if path_or_url.starts_with("file://") {
        let url = Url::parse(path_or_url)?;
        let path = url.to_file_path().map_err(|_| "Invalid file path in URL")?;
        let content = std::fs::read_to_string(path)?;
        Ok(content)
    } else {
        let content = std::fs::read_to_string(path_or_url)?;
        Ok(content)
    }
}

/// Parse the PAC proxy result and extract the primary proxy URL.
pub fn parse_proxy_result(result: &str) -> Option<String> {
    for part in result.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let parts: Vec<&str> = part.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        let proxy_type = parts[0].to_uppercase();
        let addr = parts[1];

        if proxy_type == "PROXY" || proxy_type == "HTTP" || proxy_type == "HTTPS" {
            if addr.starts_with("http://") || addr.starts_with("https://") {
                return Some(addr.to_string());
            } else {
                return Some(format!("http://{}", addr));
            }
        } else if proxy_type == "SOCKS" || proxy_type == "SOCKS5" {
            return Some(format!("socks5://{}", addr));
        } else if proxy_type == "SOCKS4" {
            return Some(format!("socks4://{}", addr));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_proxy_result() {
        assert_eq!(parse_proxy_result("DIRECT"), None);
        assert_eq!(
            parse_proxy_result("PROXY proxy.com:8080"),
            Some("http://proxy.com:8080".to_string())
        );
        assert_eq!(
            parse_proxy_result("PROXY proxy.com:8080; DIRECT"),
            Some("http://proxy.com:8080".to_string())
        );
        assert_eq!(
            parse_proxy_result("HTTP secure.com:443"),
            Some("http://secure.com:443".to_string())
        );
        assert_eq!(
            parse_proxy_result("SOCKS socks.com:1080"),
            Some("socks5://socks.com:1080".to_string())
        );
        assert_eq!(
            parse_proxy_result("SOCKS5 socks.com:1080"),
            Some("socks5://socks.com:1080".to_string())
        );
        assert_eq!(
            parse_proxy_result("SOCKS4 socks.com:1080"),
            Some("socks4://socks.com:1080".to_string())
        );
    }

    #[test]
    fn test_pac_js_helpers() {
        let pac = r#"
            function FindProxyForURL(url, host) {
                if (isPlainHostName(host)) return "DIRECT";
                if (dnsDomainIs(host, "google.com")) return "PROXY google-proxy.com:80";
                if (shExpMatch(host, "*.internal.net")) return "PROXY internal-proxy.com:8080";
                if (isInNet(host, "10.0.0.0", "255.0.0.0")) return "PROXY ten-net.com:80";
                return "DIRECT";
            }
        "#;
        let resolver = PacResolver::new(pac.to_string()).unwrap();

        // isPlainHostName
        assert_eq!(
            resolver.find_proxy_for_url("http://localhost", "localhost"),
            Some("DIRECT".to_string())
        );

        // dnsDomainIs
        assert_eq!(
            resolver.find_proxy_for_url("http://www.google.com", "www.google.com"),
            Some("PROXY google-proxy.com:80".to_string())
        );
        assert_eq!(
            resolver.find_proxy_for_url("http://google.com", "google.com"),
            Some("PROXY google-proxy.com:80".to_string())
        );

        // shExpMatch
        assert_eq!(
            resolver.find_proxy_for_url("http://sub.internal.net", "sub.internal.net"),
            Some("PROXY internal-proxy.com:8080".to_string())
        );

        // isInNet (using raw IP string)
        assert_eq!(
            resolver.find_proxy_for_url("http://10.1.2.3", "10.1.2.3"),
            Some("PROXY ten-net.com:80".to_string())
        );
        assert_eq!(
            resolver.find_proxy_for_url("http://192.168.1.1", "192.168.1.1"),
            Some("DIRECT".to_string())
        );
    }
}
