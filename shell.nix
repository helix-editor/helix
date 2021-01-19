{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    rust-bin.nightly.latest.rust
    lld_10
    # pkgconfig
  ];
  RUSTFLAGS = "-C link-arg=-fuse-ld=lld";
  RUST_BACKTRACE = "1";
}

