{
  description = "Tree-sitter grammar and queries for Raspberry Pico PIO assembly";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable-small";

  outputs =
    { self, ... }@inputs:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];
      forEachSupportedSystem =
        f:
        inputs.nixpkgs.lib.genAttrs supportedSystems (
          system: f (import inputs.nixpkgs { inherit system; })
        );

      mkPioLsp = pkgs: pkgs.callPackage ./package.nix { };
    in
    {
      packages = forEachSupportedSystem (pkgs: {
        default = mkPioLsp pkgs;
      });
      devShells = forEachSupportedSystem (pkgs: {
        default = pkgs.mkShell {
          inputsFrom = [ self.packages.${pkgs.stdenv.hostPlatform.system}.default ];

          packages = with pkgs; [
            pioasm
            clang
            cmake
            clippy
            rust-analyzer
            rustfmt
            pioasm

            (python3.withPackages (
              p: with p; [
                pdfplumber
              ]
            ))
          ];
        };
      });
    };
}
