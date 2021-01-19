{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    (rust-bin.nightly.latest.rust.override { extensions = ["rust-src"]; })
    lld_10
    # pkgconfig
  ];
  RUSTFLAGS = "-C link-arg=-fuse-ld=lld -C target-cpu=native";
  RUST_BACKTRACE = "1";
}

