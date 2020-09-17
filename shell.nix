let
  mozilla = import (builtins.fetchTarball
    "https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz");
  pkgs = import <nixpkgs> { overlays = [ mozilla ]; };
in with pkgs;
let
  rust = (rustChannelOf {
    channel = "stable";
    date = "2020-08-03";
  }).rust.override { extensions = [ "rust-src" ]; };
in mkShell {
  buildInputs = [ clang rust ];
  LIBCLANG_PATH = "${llvmPackages.libclang}/lib";
}
