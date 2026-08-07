[![Build](https://github.com/nmetschke/piolsp/actions/workflows/test.yml/badge.svg)](https://github.com/nmetschke/piolsp/actions/workflows/test.yml)

# LSP Server for PIO assembly files

## About

Small [LSP](https://microsoft.github.io/language-server-protocol) Server for Raspberry Pi [PIO assembly files](https://pip-assets.raspberrypi.com/categories/609-microcontroller-boards/documents/RP-009085-KB-2-raspberry-pi-pico-c-sdk.pdf#page=56). Recommended to use in conjunction with [tree-sitter-pio](https://github.com/nmetschke/tree-sitter-pio) for syntax highlighting and other [tree-sitter](https://tree-sitter.github.io/tree-sitter) queries.

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

### Building from source

```sh
  cargo build --release
```

### Nix flake

Add the input

```nix
{
  inputs.piolsp = {
    url = "github:nmetschke/piolsp";
    inputs.nixpkgs.follows = "nixpkgs";
  };
}
```

and use the package with

```nix
  inputs.piolsp.packages.${pkgs.stdenv.hostPlatform.system}.default
```

.

## Usage

### LSP Server

By default, pioasm starts as an LSP server and should work with most LSP clients.
It does however require the client to support UTF-8 position encoding.
The default UTF-16 encoding is not (yet) supported.

### With Helix

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

and run `hx --grammar fetch && hx --grammar build`.

### Helix with Nix + home-manager

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
{ pkgs, inputs, ... }:
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
            command = lib.getExe inputs.piolsp.packages.${pkgs.stdenv.hostPlatform.system}.default;
            args = [ "--pioasm" (lib.getExe pkgs.pioasm) ]; # if pioasm is not already in $PATH
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

Running `hx --grammar fetch && hx --grammar build` is still necessary to build the PIO grammar.

### Latest Helix with Nix + home-manager

In case you are using the upstream helix [flake.nix](https://github.com/helix-editor/helix/flake.nix), the PIO grammar can also be directly included into helix by building the package like this:

```nix
  programs.helix = {
    package = inputs.helix.packages.${pkgs.stdenv.hostPlatform.system}.helix.override {
      grammarOverlays = [
        (
          final: prev:
          let
            pioGrammar = inputs.tree-sitter-pio.packages.${pkgs.stdenv.hostPlatform.system}.default;
            sharedLibExt = pkgs.stdenv.hostPlatform.extensions.sharedLibrary;
          in
          {
            # helix expects <grammar>.so instead of parser
            # see https://github.com/NixOS/nixpkgs/blob/1f5a62ea007185cc75f0596668d6824a052f06ed/pkgs/by-name/he/helix/package.nix#L77-L82
            pio = pkgs.runCommand "helix-tree-sitter-pio" { } ''
              install -D ${pioGrammar}/parser $out/${pioGrammar.language}.${sharedLibExt}
              ${lib.getExe pkgs.removeReferencesTo} -t ${pioGrammar} $out/${pioGrammar.language}.${sharedLibExt}
            '';
          }
        )
      ];
    };

    # use the same "helix.languages" and "helix/runtime/queries/pio" configuration as above
    # do not include "helix.grammar" with this
  };
```

This avoids having to run `hx --grammar fetch && hx --grammar build` but will cause a rebuild and prevent usage of the helix [cachix](https://helix.cachix.org) instance.

.
