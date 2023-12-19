{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
  };
  outputs = {
    self,
    nixpkgs,
    flake-utils,
    rust-overlay,
  }:
    flake-utils.lib.eachDefaultSystem
    (
      system: let
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rustToolchain = pkgs.pkgsBuildHost.rust-bin.nightly.latest.default.override {
          extensions = ["rust-src" "rust-analyzer" "rustc-codegen-cranelift-preview"];
        };
      in
        with pkgs; {
          devShells.default = mkShell {
            nativeBuildInputs = with pkgs; [rustToolchain pkg-config];
            buildInputs = with pkgs; [openssl clang cargo-release];
            RUST_BACKTRACE = "1";
          };
        }
    );
}
