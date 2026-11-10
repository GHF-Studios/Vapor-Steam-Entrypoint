# Vapor Steam Entrypoint

Vapor Steam Entrypoint is a small locator shipped in the Loo Cast Steam
Packagepack depot. It adapts Steam launch choices to a separately installed
Vapor Home. It is not the Vapor runtime and never owns Steam-root state.

Steam has two launch choices per target:

| Choice | Linux target | Windows target |
| --- | --- | --- |
| Play Loo-Cast (default) | `bin/x86_64-unknown-linux-gnu/vapor-entrypoint` | `bin\\x86_64-pc-windows-gnullvm\\vapor-entrypoint.exe` |
| Choose an app | `bin/x86_64-unknown-linux-gnu/vapor-entrypoint chooser` | `bin\\x86_64-pc-windows-gnullvm\\vapor-entrypoint.exe chooser` |

The Steam payload also contains the Packagepack manifest, the matching
Packagepack executable, and only explicitly required runtime sidecars. The
locator is the sole Vapor-owned Steam payload file.

## Vapor Home discovery

The locator resolves a valid `Vapor-Home.toml` from, in order:

1. `VAPOR_HOME`;
2. the standard `%USERPROFILE%\\Vapor` or `$HOME/Vapor` directory;
3. the installation containing `vapor` found on `PATH`.

It then finds the host-target binaries under `Vapor Home/bin/<target>`. The
Steam app root is never assigned to `VAPOR_HOME`. Child processes run with the
resolved home as their working directory and receive that exact `VAPOR_HOME`.
Logs are stored at `VAPOR_HOME/data/logs/entrypoint.log`.

When Vapor cannot be found, the locator displays a visible message pointing to
`https://vapor.ghf-studios.site/` and exits. It does not install Vapor or
create a replacement runtime in the Steam app.

## Launch routing

| Steam mode | Route inside Vapor Home |
| --- | --- |
| no arguments, `play`, or `game` | Verify Player setup, ensure the shared Client service, then run `vapor launch selected` |
| `chooser` | Open `vapor-gui --mode chooser` |
| `launcher`, `studio`, `ide` | Open the corresponding Vapor GUI executable |
| `installer` | Open the Installer GUI, with the Vapor launch script as recovery fallback |
| `shell` or another CLI command | Open the Vapor launch script in a terminal |

The Steam locator owns only app-root discovery, Vapor Home discovery, handoff,
argument forwarding, and logs. Vapor owns setup, selected compositions,
content, tools, and all persistent state.
