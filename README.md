# Vapor Steam Entrypoint

Vapor Steam Entrypoint adapts Steam launches to the installed Vapor app. Its
default launch starts the app manifest's default playable composition directly.
The optional `chooser` route opens Launcher, Studio, IDE, Installer, and Shell.

Steam has two intended launch choices per target:

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
Steam
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

Entrypoint logs go to `<app-root>/.vapor/logs/entrypoint.log`.

This project owns app-root discovery, game and GUI handoff, terminal recovery
launches, argument forwarding, and entrypoint logging. Vapor owns shared
operations; Vapor-CLI and Vapor-GUI provide terminal and graphical facades.
