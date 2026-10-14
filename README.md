# Vapor Entrypoint

Vapor Entrypoint is the Steam-facing launch adapter for the installed Vapor
app. The `studio` route starts Vapor Studio directly; explicit recovery routes
continue through the terminal wrapper.

Steam starts `vapor-entrypoint[.exe]` with `studio` to open the sibling
`vapor-studio[.exe]` in the same target directory. Steam's default launch
configuration should use this route. `play` asks Studio to build and run the
selected profile's composition; if no selection resolves, Studio remains open
and explains the issue. `shell` and `installer` retain terminal-backed
recovery behavior.

Runtime paths:

```text
Steam
└── bin/<target>/vapor-entrypoint[.exe]
    ├── studio -> bin/<target>/vapor-studio[.exe]
    ├── play -> bin/<target>/vapor-studio[.exe] --play
    └── recovery modes -> Konsole or cmd
        └── bin/vapor-launch.sh or bin\vapor-launch.cmd
            └── vapor or vapor-installer
```

Logs go to:

```text
<app-root>/.vapor/logs/entrypoint.log
```

This project owns app-root discovery, Studio handoff, terminal recovery
launches, argument forwarding, and entrypoint logging. Vapor Core and Vapor
Shell remain responsible for content and runtime behavior.
