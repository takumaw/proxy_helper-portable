# Design

## Goal

`proxy_helper` is a lightweight Linux command-line utility that retrieves
Linux system proxy configurations (GNOME, KDE) or local configuration files
and exports them as shell environment variables.

It propagates settings to terminal environments, supporting Bourne-style
shells (e.g., `bash`, `zsh`), C-style shells (e.g., `csh`, `tcsh`), Fish
Shell, and PowerShell (pwsh).

## Non-goals

This project does not:
* Implement or host a proxy server.
* Support operating systems other than Linux (macOS has native mechanisms and
  is not targeted).
* Perform network requests other than resolving PAC (Proxy Auto-Configuration)
  files.

## Architecture & source layout

The program is structured in Rust, separating entry points, CLI parsing logic,
system configs retrieval, and PAC script evaluation:

```text
src/
  ├── main.rs          # Entry point, argument parsing, & shell formatting
  ├── system_proxy.rs  # Linux settings readers (gsettings, KDE config, TOML config)
  ├── pac.rs           # Embedded JavaScript PAC evaluator using rquickjs
  └── no_proxy.rs      # Candidates extraction logic from PAC scripts
```

### Components

* **`main` (`main.rs`)**: CLI entry point. Parses arguments, handles shell
  style detection, clean bypass exception entries, and outputs final shell scripts.
* **`system_proxy` (`system_proxy.rs`)**: Resolves configurations from
  environment variables, user configuration file (`~/.config/proxy_helper/config.toml`),
  system-wide configuration file (`/etc/proxy_helper/config.toml`), GNOME gsettings,
  or KDE kioslaverc settings.
* **`pac` (`pac.rs`)**: Hosts the embedded `rquickjs` context, loads the PAC script,
  injects Javascript helper APIs (e.g., `dnsResolve`), and evaluates proxy rules.
* **`no_proxy` (`no_proxy.rs`)**: Extracts potential host strings inside PAC
  files and filters them using the JS engine to populate `no_proxy`.

## Command line & PAC resolution

### Command-line options

* `-c`, `--csh`: Formats output for C-style shells (`csh`, `tcsh`).
* `-s`, `--sh`: Formats output for Bourne-style shells (`sh`, `bash`, `zsh`).
* `-f`, `--fish`: Formats output for Fish Shell.
* `-w`, `--powershell`: Formats output for PowerShell (pwsh).
* `-p`, `--pac`: Explicitly provides custom PAC file path or URL.
* `-u <url>`, `--url <url>`: Evaluation target URL used with PAC (defaults to `www.google.com`).

If no shell option is provided, the tool checks the `$SHELL` environment
variable to determine whether to output `setenv` (for `csh`/`tcsh`), `set -gx`
(for `fish`), `$env:` (for `pwsh`), or `export` syntax. It defaults to Bourne-style.

## Static binary & compatibility

To achieve maximum startup performance and prevent library loading overheads,
the binary is compiled statically targeting the `x86_64-unknown-linux-musl` target.
This avoids glibc runtime dependency mismatches across different Linux distributions.
