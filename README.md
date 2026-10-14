# Vapor Steam Entrypoint

Vapor Steam Entrypoint is the Steam-facing launch adapter for the installed
Vapor app. It dispatches Studio and Installer into Vapor-GUI and preserves
terminal access through Vapor-CLI.

Steam starts `vapor-entrypoint[.exe]` with `studio` to open the sibling
`vapor-gui[.exe]` in the same target directory. Steam's launch options keep
their existing arguments. `play` asks Studio to run the selected composition;
`installer` opens the GUI installer when available and falls back to the
precompiled terminal installer during recovery. `shell` opens Vapor-CLI.

Runtime paths:

```text
Steam
└── bin/<target>/vapor-entrypoint[.exe]
    ├── studio -> vapor-gui[.exe] --mode studio
    ├── play -> vapor-gui[.exe] --mode play
    ├── installer -> vapor-gui[.exe] --mode installer
    └── shell -> vapor[.exe]
        └── recovery uses vapor-launch.sh or vapor-launch.cmd
            └── vapor-installer
```

Logs go to `<app-root>/.vapor/logs/entrypoint.log`.

This project owns app-root discovery, GUI handoff, terminal recovery launches,
argument forwarding, and entrypoint logging. Vapor owns shared operations;
Vapor-CLI and Vapor-GUI provide the terminal and graphical facades.
