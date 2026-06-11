# proxy_helper

## Overview

The `proxy_helper` utility reads the proxy settings from Linux system settings
(GNOME, KDE), a custom configuration file, or a Proxy Auto-Configuration (PAC)
file and constructs the `http_proxy`, `https_proxy`, `ftp_proxy`, and
`no_proxy` environment variables respectively.

This utility is written in Rust for high performance, low memory footprint,
and instant startup, making it safe to run inside `.bashrc` or `.zshrc`.
It features full support for executing JavaScript-based PAC scripts using
an embedded lightweight JS engine (`rquickjs`).

    $ proxy_helper -s
    http_proxy="http://YOUR_HTTP_PROXY_SERVER:PORT"; export http_proxy; https_proxy="http://YOUR_HTTPS_PROXY_SERVER:PORT"; export https_proxy; ftp_proxy="http://YOUR_FTP_PROXY_SERVER:PORT"; export ftp_proxy; no_proxy="NO_PROXIES"; export no_proxy;

## Requirements

* Linux operating system.

## How to install

Download the pre-compiled statically linked binary
(`proxy_helper-vX.Y.Z-linux-amd64.tar.gz`) from GitHub Releases.

Since this utility is intended to be run only via shell initialization scripts,
it is recommended to place the binary under a `libexec` directory to keep it
out of your standard executable `$PATH`.

Because this tool resolves proxies tied to the machine environment, it is
recommended to install it system-wide:

* For system-wide installation (recommended): `/usr/local/libexec/proxy_helper`
* For user-local installation: `~/.local/libexec/proxy_helper`

For the examples below, we assume the binary is placed at the recommended
system-wide path `/usr/local/libexec/proxy_helper`.

## How it works

In your `/etc/profile` for Bash, or `/etc/zprofile` for Zsh, add the
following code snippet:

    if [ -x /usr/local/libexec/proxy_helper ]; then
        eval `/usr/local/libexec/proxy_helper -s`
    fi

Change the path to the binary if you installed it to another directory.

For Csh or Tcsh users, put the following code to e.g. `/etc/csh.login`:

    if ( -x /usr/local/libexec/proxy_helper ) then
        eval `/usr/local/libexec/proxy_helper -c`
    endif

For Fish users, put the following code to `~/.config/fish/config.fish`:

    if test -x /usr/local/libexec/proxy_helper
        eval (/usr/local/libexec/proxy_helper -f)
    end

For PowerShell users, put the following code in your profile script:

    if (Test-Path /usr/local/libexec/proxy_helper) {
        /usr/local/libexec/proxy_helper -w | Invoke-Expression
    }

That's all set! All your newly invoked shells now have proxy environment
variables set.

**Tip: Reloading proxy settings**

If you switch networks while a terminal session is open, you will need to
re-evaluate the proxy settings. You can define an alias in your shell profile
to make this easier:

```bash
alias reload_proxy="eval \`/usr/local/libexec/proxy_helper -s\`"
```

## Troubleshooting

### No proxy environment variables are set

If `proxy_helper` does not output any environment variables when run, check:

1. **Desktop Proxy Settings**: Ensure that at least one proxy is configured and
   enabled in your Linux desktop settings (GNOME Network Proxy or KDE Proxy
   Control Module). If no proxies are active, `proxy_helper` exits silently.
2. **Configuration File**: Make sure your `~/.config/proxy_helper/config.toml`
   is formatted correctly if you are overriding system settings.
3. **Manual PAC Evaluation**: Test resolution for a target URL by passing the
   PAC URL directly via the `-p` (or `--pac`) option:
   ```bash
   /usr/local/libexec/proxy_helper -p http://example.com/proxy.pac -u https://www.google.com
   ```

## Documentation

For detailed information on available options and how `no_proxy` rules are
evaluated, please refer to the manual page:

```bash
man proxy_helper
```

## Author

* Takuma WATANABE

## License

Released under the [MIT License](LICENSE).
