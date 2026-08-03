{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "piolsp";
  version = "0.1.0";

  src = lib.sources.cleanSource ./.;
  cargoLock = {
    lockFile = ./Cargo.lock;
  };

  checkFlags = [ "--skip=formatter::test::format_pico_examples" ]; # requires submodule
}
