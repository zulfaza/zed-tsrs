# tsrs for Zed

This fork of [zed-extensions/tsgo](https://github.com/zed-extensions/tsgo)
uses the [tsrs](https://github.com/oxc-project/tsrs) Rust language server.
It supports TypeScript, TSX, JavaScript, and JSX.

## Install

1. Install Rust with [rustup](https://rustup.rs/).
2. Clone this repository:

   ```sh
   git clone https://github.com/zulfaza/zed-tsrs.git
   ```

3. In Zed, run `zed: install dev extension` and select the `zed-tsrs` directory.
4. Add these entries to your Zed settings:

```json
{
  "languages": {
    "TypeScript": {
      "language_servers": ["tsrs", "!typescript-ls", "!tsgo", "!vtsls", "!typescript-language-server", "..."]
    },
    "TSX": {
      "language_servers": ["tsrs", "!typescript-ls", "!tsgo", "!vtsls", "!typescript-language-server", "..."]
    },
    "JavaScript": {
      "language_servers": ["tsrs", "!typescript-ls", "!tsgo", "!vtsls", "!typescript-language-server", "..."]
    },
    "JSX": {
      "language_servers": ["tsrs", "!typescript-ls", "!tsgo", "!vtsls", "!typescript-language-server", "..."]
    }
  }
}
```

The extension installs the latest `tsrs` npm package in its own work directory.
It uses Zed's Node runtime to run `tsrs/bin/tsrs --lsp --stdio`.
The npm launcher selects the native binary, including the Linux libc variant.
Your platform must be supported by the selected npm release.

## Select a version

To use a fixed version, add:

```json
{
  "lsp": {
    "tsrs": {
      "settings": {
        "package_version": "0.18.0-ts7.1.0-dev.20260929"
      }
    }
  }
}
```

## Use a local binary

Set an absolute path to a native `tsrs` binary to skip the npm installation:

```json
{
  "lsp": {
    "tsrs": {
      "binary": {
        "path": "/absolute/path/to/tsrs",
        "arguments": ["--lsp", "--stdio"]
      }
    }
  }
}
```

`binary.env` sets environment variables for the server.
`initialization_options` sets LSP initialization options.
`settings.typescript` and `settings.javascript` override the server defaults.
Inlay hints and code lens keep the defaults from the source extension.

## Build

```sh
cargo fmt --check
cargo clippy --locked --target wasm32-wasip2 -- -D warnings
cargo build --locked --release --target wasm32-wasip2
```

The build produces `target/wasm32-wasip2/release/tsrs.wasm`.
Zed builds the extension when you install it as a dev extension.

## Checks

The WebAssembly release build, formatting check, and Clippy check passed.
A direct LSP check with the npm release above returned hover text and TS2322
for a number assigned to a string variable.
The server accepted the shutdown request, then exited with code 1 and
`context canceled` after the exit notification.
The extension was installed in Zed on macOS. Zed started the native tsrs server.

## Source

This repository is a fork of [zed-extensions/tsgo](https://github.com/zed-extensions/tsgo).
The source extension's Apache-2.0 license is retained.
Install this fork as a dev extension. It is not in the Zed extension registry.
