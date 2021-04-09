{
  description = "A post-modern text editor.";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    naersk.url = "github:nmattia/naersk";
  };

  outputs = inputs@{ self, nixpkgs, naersk, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; overlays = [ rust-overlay.overlay ]; };
        naerskLib = pkgs.callPackage naersk {
          inherit (pkgs.rust-bin.stable.latest.default) rustc cargo;
        };
      in rec {
        packages.helix = naerskLib.buildPackage {
          pname = "helix";
          root = ./.;
        };
        defaultPackage = packages.helix;
        devShell = pkgs.callPackage ./shell.nix {};
      });
}
