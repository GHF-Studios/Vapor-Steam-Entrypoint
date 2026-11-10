# Vapor Steam Entrypoint

This is a Vapor-owned external launch adapter prototype. It is intended to
live under Vapor home and is not part of the Loo-Cast Steam content mount.
The current content-only Steam depot layout does not yet provide a direct
Steam launch-option handoff to this executable; that integration needs a
separate mechanism outside the Steam product payload.

The eventual Vapor-owned launch choices for a mounted Loo-Cast product are:

| Choice | Linux target | Windows target |
| --- | --- | --- |
| Play Loo-Cast (default) | `bin/x86_64-unknown-linux-gnu/vapor-entrypoint` | `bin\x86_64-pc-windows-gnullvm\vapor-entrypoint.exe` |
| Choose an app | `bin/x86_64-unknown-linux-gnu/vapor-entrypoint chooser` | `bin\x86_64-pc-windows-gnullvm\vapor-entrypoint.exe chooser` |

The direct route starts the shared Client service and asks Vapor to launch the
default composition. The chooser route dispatches to distinct Launcher, Studio,
IDE, and Installer executables; Shell opens Vapor-CLI in a terminal. Installer
also retains its recovery route when the shared service cannot start.

Runtime paths:

```text
<Vapor home>
└── bin/<target>/vapor-entrypoint[.exe]
    ├── (no arguments) -> vapor[.exe] launch default
    ├── chooser -> vapor-gui[.exe] --mode chooser
    │   ├── launcher -> vapor-launcher[.exe] --mode launcher
    │   ├── studio -> vapor-studio[.exe]
    │   ├── ide -> vapor-ide[.exe]
    │   ├── installer -> vapor-installer-gui[.exe]
    │   └── shell -> terminal -> vapor[.exe]
    ├── installer -> vapor-installer-gui[.exe]
    └── shell -> terminal -> vapor[.exe]
```

The entrypoint discovers Vapor under `%USERPROFILE%\\Vapor` on Windows or
`$HOME/Vapor` on Linux. `VAPOR_HOME` may select a custom standalone install.
The Steam app root is a mounted content source. It contains `App.vapor.toml`
and the three first-party content items, with no Vapor binaries or mutable
runtime state. Entrypoint logs go to
`<vapor-home>/data/logs/entrypoint.log`.

This project owns app-root discovery, game and GUI handoff, terminal recovery
launches, argument forwarding, and entrypoint logging. Vapor owns shared
operations; Vapor-CLI and Vapor-GUI provide terminal and graphical facades.
