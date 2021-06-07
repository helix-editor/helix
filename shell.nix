{ lib, stdenv, pkgs }:

pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    (rust-bin.stable.latest.default.override { extensions = ["rust-src"]; })
    lld_10
    lldb
    # pythonPackages.six
    stdenv.cc.cc.lib
    # pkg-config
  ];
  RUSTFLAGS = "-C link-arg=-fuse-ld=lld -C target-cpu=native";
  RUST_BACKTRACE = "1";
  # https://github.com/rust-lang/rust/issues/55979
  LD_LIBRARY_PATH = lib.makeLibraryPath (with pkgs; [
    stdenv.cc.cc.lib
  ]);

  shellHook = ''
    export HELIX_RUNTIME=$PWD/runtime
  '';
}
