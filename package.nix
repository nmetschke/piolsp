{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "piolsp";
  version = "0.1.0";

  src = lib.sources.cleanSource ./.;
  cargoLock = {
    lockFile = ./Cargo.lock;
  };

  checkFlags = [ "--skip=formatter::test::format_pico_examples" ]; # requires submodule

  meta = {
    description = "LSP Server for Raspberry Pi PIO assembly";
    homepage = "https://github.com/nmetschke/piolsp";
    mainProgram = "piolsp";
    license = lib.licenses.mit;
  };
}
