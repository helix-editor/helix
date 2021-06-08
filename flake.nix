{
  description = "A post-modern text editor.";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    naersk.url = "github:nmattia/naersk";
    helix = {
      flake = false;
      url = "https://github.com/helix-editor/helix";
      type = "git";
      submodules = true;
    };
  };

  outputs = inputs@{ self, nixpkgs, naersk, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; overlays = [ rust-overlay.overlay ]; };
        rust = (pkgs.rustChannelOf {
            date = "2021-05-01";
            channel = "nightly";
        }).minimal; # cargo, rustc and rust-std
        naerskLib = naersk.lib."${system}".override {
          # naersk can't build with stable?!
          # inherit (pkgs.rust-bin.stable.latest) rustc cargo;
          rustc = rust;
          cargo = rust;
        };
      in rec {
        packages.helix = naerskLib.buildPackage {
          pname = "helix";
          root = inputs.helix;
        };
        defaultPackage = packages.helix;
        devShell = pkgs.callPackage ./shell.nix {};
      });
}
