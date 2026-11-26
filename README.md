# Retired Steam entrypoint

Vapor Steam Entrypoint adapts Steam launches to the installed Vapor app. Its
default launch starts the app manifest's default playable composition directly.
The optional `creator` route starts the standalone Vapor installation, or opens
the Downloads page when Vapor is not installed. The `chooser` route remains
available for local development and recovery.

Steam has two intended launch choices per target:

| Choice | Linux target | Windows target |
| --- | --- | --- |
| Play Loo-Cast (default) | `bin/x86_64-unknown-linux-gnu/vapor-entrypoint` | `bin\x86_64-pc-windows-gnullvm\vapor-entrypoint.exe` |
| Create with Vapor | `bin/x86_64-unknown-linux-gnu/vapor-entrypoint creator` | `bin\x86_64-pc-windows-gnullvm\vapor-entrypoint.exe creator` |

The direct route starts the shared Client service and asks Vapor to launch the
default composition. Creator resolves the active release under Vapor User Data
and starts its chooser without inheriting the Steam payload's `VAPOR_HOME`; if
there is no active release, it opens the Downloads page. Before handoff it also
clears any inherited `VAPOR_USER_DATA_ROOT`, so the installed Vapor resolves its
own canonical data root. The bundled chooser route remains for local
development and recovery; the Steam depot omits the chooser and creator GUI
binaries.

Runtime paths:

```text
Steam
└── bin/<target>/vapor-entrypoint[.exe]
    ├── (no arguments) -> vapor[.exe] launch default
    └── creator -> Vapor User Data/apps/vapor/active release chooser
        └── no active release -> https://vapor.ghf-studios.site/downloads
```

Entrypoint logs go to `<Vapor User Data>/logs/entrypoint.log`.

This project owns app-root discovery, game and creator handoff, terminal
recovery launches, argument forwarding, and entrypoint logging. The Steam
payload retains only the player entrypoint, Vapor CLI, client-service bootstrap,
required runtime libraries, and packaged Loo-Cast runtime closure. Vapor owns
shared operations; Vapor-CLI and Vapor-GUI provide terminal and graphical
facades in the separate installation.
