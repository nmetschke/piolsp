[![Build](https://github.com/nmetschke/piolsp/actions/workflows/test.yml/badge.svg)](https://github.com/nmetschke/piolsp/actions/workflows/test.yml)

# Overview

Small LSP Server for Raspberry Pi Pio Assembly. Recommended to use in conjunction with [tree-sitter-pio](https://github.com/nmetschke/tree-sitter-pio) for syntax highlighting and other tree-sitter queries.

## Features

- **#**: Working
- **~**: Partially Working
- **X**: Not yet implemented but planned

| Feature                       | Status | Notes                                                                                                                                                                                   |
| ----------------------------- | ------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Diagnostics                   | **#**  | Requires pioasm to be available.                                                                                                                                                        |
| Go to definition / references | **#**  | For .define statements and labels                                                                                                                                                       |
| Inlay hints                   | **#**  | Instruction address / count, label address                                                                                                                                              |
| Symbol renaming               | **#**  |                                                                                                                                                                                         |
| Hover doc for instructions    | **#**  | Auto generated from [Raspberry Pi Pico-series C/C++ SDK](https://pip-assets.raspberrypi.com/categories/609-microcontroller-boards/documents/RP-009085-KB-2-raspberry-pi-pico-c-sdk.pdf) |
| Formatting                    | **~**  | WIP                                                                                                                                                                                     |
| Autocomplete                  | **X**  |                                                                                                                                                                                         |

## Installation

### Nix flake

```nix
{
  inputs.piolsp = {
    url = "github:nmetschke/piolsp";
    inputs.nixpkgs.follows = "nixpkgs";
  };
}
```

### Building from source

```sh
  cargo build --release
```

## Usage

## LSP Server

By default, pioasm starts as an LSP server and should work with most LSP clients.
It does however require the client to support UTF-8 position encoding.
The default UTF-16 encoding is not (yet) supported.

## With Helix

Add this to `~/.congig/helix/languages.toml`.

```toml
[language-server.piolsp]
# args = ["--pioasm", "<path_to_pioasm>"] # optional, searches in $PATH by default
command = "piolsp"


[[language]]
comment-tokens = ";"
file-types = ["pio"]
language-servers = ["piolsp"]
name = "pio"

# if the tree-sitter-pio grammar is used
scope = "source.pio"

[grammar.source]
git = "https://github.com/nmetschke/tree-sitter-pio"
rev = "bc094f6df53f4311468ded14be1c37e8dfdc601c"
```

## Helix with Nix + home-manager

Example [Helix](https://github.com/helix-editor/helix) configuration for Nix with [tree-sitter](https://tree-sitter.github.io/tree-sitter) grammar.

Add the tree-sitter grammar and piolsp to inputs.

`flake.nix`:

```nix
inputs = {
  tree-sitter-pio = {
    url = "github:nmetschke/tree-sitter-pio";
    inputs.nixpkgs.follows = "nixpkgs";
  };
  piolsp = {
    url = "github:nmetschke/piolsp";
    inputs.nixpkgs.follows = "nixpkgs";
  };
};
```

and configure helix to use them

```nix
{ inputs, ... }:
{
  programs.helix = {
    languages = {
      language = [
        {
          name = "pio";
          scope = "source.pio";
          file-types = [ "pio" ];
          comment-tokens = ";";
          language-servers = [ "piolsp" ];
        }
      ];
      language-server = [
        {
          piolsp = {
            command = lib.getExe inputs.piolsp;
            # optional: pioasm is used for diagnostics, not needed if pioasm is already in $PATH
            args = [
              "--pioasm"
              (lib.getExe pkgs.pioasm)
            ];
          };
        }
      ];
      grammar = [
        {
          name = "pio";
          source.path = inputs.tree-sitter-pio;
        }
      ];
    };
  };

  # link the queries
  xdg.configFile."helix/runtime/queries/pio".source = "${inputs.tree-sitter-pio}/queries";
}
```
