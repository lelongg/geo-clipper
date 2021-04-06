let
  rust-overlay = builtins.fetchTarball {
    url =
      "https://github.com/oxalica/rust-overlay/tarball/611e6213c5563a3f46a57c600c70e0f0fd2811f3";
    sha256 = "sha256:1z9yv2wcxpzf7y4lsv21lrvzwcvsfpgfjqsg53m5z3h5pdvap26g";
  };
  pkgs = import <nixpkgs> { overlays = [ (import (rust-overlay)) ]; };
in with pkgs;
let
  rustChannel = rustChannelOf { channel = "1.51.0"; };
  rustStable = rustChannel.rust.override { extensions = [ "rust-src" ]; };
  rustPlatform = makeRustPlatform {
    rustc = rustStable;
    cargo = rustStable;
  };
in mkShell {
  buildInputs = [ clang rustStable openssl pkgconfig ];
  LIBCLANG_PATH = "${llvmPackages.libclang}/lib";
}
