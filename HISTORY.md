# History

## 0.1.0

* Statically compiled Linux x86_64 binary targeting `musl-libc` (highly responsive, low memory footprint).
* Full support for evaluating JS-based Proxy Auto-Configuration (PAC) scripts inside an embedded `rquickjs` engine.
* Network proxy resolution support on Linux system settings (GNOME via `gsettings` and KDE via `kioslaverc`).
* Support for user-local (`~/.config/proxy_helper/config.toml`) and system-wide (`/etc/proxy_helper/config.toml`) configuration files.
* Advanced exceptions validation supporting IPv4, IPv6 (with brackets stripping), and CIDR ranges in `no_proxy` generation.
* Interactive shell support generating output commands for Bourne shells (e.g., `bash`, `zsh`), C-shells (`csh`, `tcsh`), Fish, and PowerShell.
