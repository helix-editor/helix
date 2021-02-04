{
  description = "A post-modern text editor.";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay/a9b13ba83eaf2d07ae955a45b15fd96aa6994b70";
  };

  outputs = inputs@{ self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; overlays = [ rust-overlay.overlay ]; };
      in rec {
        # packages.helix = pkgs.callPackage ./default.nix {};
        # defaultPackage = packages.helix;
        devShell = pkgs.callPackage ./shell.nix {};
      });
}
