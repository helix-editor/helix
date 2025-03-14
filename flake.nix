{
  description = "A post-modern text editor.";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    rust-overlay,
    ...
  }: let
    gitRev = self.rev or self.dirtyRev or null;
  in
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [(import rust-overlay)];
      };

      # Get Helix's MSRV toolchain to build with by default.
      msrvToolchain = pkgs.pkgsBuildHost.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      msrvPlatform = pkgs.makeRustPlatform {
        cargo = msrvToolchain;
        rustc = msrvToolchain;
      };
    in {
      packages = rec {
        helix = pkgs.callPackage ./default.nix {inherit gitRev;};

        /**
        The default Helix build. Uses the latest stable Rust toolchain, and unstable
        nixpkgs.
        
        The build inputs can be overriden with the following:
         
        packages.${system}.default.override { rustPlatform = newPlatform; };
         
        Overriding a derivation attribute can be done as well:
        
        packages.${system}.default.overrideAttrs { buildType = "debug"; };
        */ 
        default = helix;
      };

      checks.helix = self.outputs.packages.${system}.helix.override {
        buildType = "debug";
        rustPlatform = msrvPlatform;
      };

      # Devshell behavior is preserved.
      devShells.default = let
        commonRustFlagsEnv = "-C link-arg=-fuse-ld=lld -C target-cpu=native --cfg tokio_unstable";
        platformRustFlagsEnv = pkgs.lib.optionalString pkgs.stdenv.isLinux "-Clink-arg=-Wl,--no-rosegment";
      in
        pkgs.mkShell
        {
          inputsFrom = [self.checks.${system}.helix];
          nativeBuildInputs = with pkgs;
            [
              lld
              cargo-flamegraph
              rust-bin.nightly.latest.rust-analyzer
            ]
            ++ (lib.optional (stdenv.isx86_64 && stdenv.isLinux) cargo-tarpaulin)
            ++ (lib.optional stdenv.isLinux lldb)
            ++ (lib.optional stdenv.isDarwin darwin.apple_sdk.frameworks.CoreFoundation);
          shellHook = ''
            export RUST_BACKTRACE="1"
            export RUSTFLAGS="''${RUSTFLAGS:-""} ${commonRustFlagsEnv} ${platformRustFlagsEnv}"
          '';
        };
    })
    // {
      overlays.default = final: prev: {
        helix = final.callPackage ./default.nix {inherit gitRev;};
      };
    };

  nixConfig = {
    extra-substituters = ["https://helix.cachix.org"];
    extra-trusted-public-keys = ["helix.cachix.org-1:ejp9KQpR1FBI2onstMQ34yogDm4OgU2ru6lIwPvuCVs="];
  };
}
