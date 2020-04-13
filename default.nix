with import <nixpkgs> { };

mkShell {
  buildInputs = [ clang cargo cmake ];
  LIBCLANG_PATH = "${llvmPackages.libclang}/lib";
}
