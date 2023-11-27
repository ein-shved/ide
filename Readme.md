# IDEM

Simple IDE manager for neovim version 0.3.0.

## About

This is hello-world project/instrument on rust for launching neovide (a neovim
gui) for previously saved sessions with
[Neovim Session Manager](https://github.com/Shatur/neovim-session-manager).

## Usage

By default the command `ide` launching the gtk window with available sessions.
You may filter, remove, open existing or open new session. It launches neovide
by default.

![Gtk UI](/pictures/screenshot_1.png)

You may control the UI by passing `UI` environment variable. Available UIs:

* `Gtk`
* `Stdout` - prints available sessions
* `Stdio` - interactive tui

You may pass name of session to arguments to run ide in-place.

## Building

### By hand

Just run

```bash
cargo build -r
```

the binary will be at `target/release/ide`

### Nix

Just run

```bash
nix build
```
the binary will be at `result/bin/ide`

### NixOS

Add this repo to your flake's inputs and add the package to
`environment.systemPackages `;


## TODO

* Dedicated server to control the vim headless instances
* Intra-server communication to attach to remote instances
* Fix Gtk window width on filtering-out all sessions
* Order session in list by date-of-opening
