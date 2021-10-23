{
  description = "A post-modern text editor.";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    nixCargoIntegration = {
      url = "github:yusdacra/nix-cargo-integration";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rustOverlay.follows = "rust-overlay";
    };
    flakeCompat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
  };

  outputs = inputs@{ self, nixCargoIntegration, ... }:
    nixCargoIntegration.lib.makeOutputs {
      root = ./.;
      buildPlatform = "crate2nix";
      renameOutputs = { "helix-term" = "helix"; };
      # Set default app to hx (binary is from helix-term release build)
      # Set default package to helix-term release build
      defaultOutputs = { app = "hx"; package = "helix"; };
      overrides = {
        crateOverrides = common: _: {
          helix-term = prev: {
            # link languages and theme toml files since helix-term expects them (for tests)
            preConfigure = "ln -s ${common.root}/{languages.toml,theme.toml} ..";
            buildInputs = (prev.buildInputs or [ ]) ++ [ common.cCompiler.cc.lib ];
          };
          # link languages and theme toml files since helix-view expects them
          helix-view = _: { preConfigure = "ln -s ${common.root}/{languages.toml,theme.toml} .."; };
          helix-syntax = _prev: {
            preConfigure = "mkdir -p ../runtime/grammars";
            postInstall = "cp -r ../runtime $out/runtime";
          };
        };
        mainBuild = common: prev:
          let
            inherit (common) pkgs lib;
            helixSyntax = lib.buildCrate {
              root = self;
              memberName = "helix-syntax";
              defaultCrateOverrides = {
                helix-syntax = common.crateOverrides.helix-syntax;
              };
              release = false;
            };
            runtimeDir = pkgs.runCommand "helix-runtime" { } ''
              mkdir -p $out
              ln -s ${common.root}/runtime/* $out
              ln -sf ${helixSyntax}/runtime/grammars $out
            '';
          in
          lib.optionalAttrs (common.memberName == "helix-term") {
            nativeBuildInputs = [ pkgs.makeWrapper ];
            postFixup = ''
              if [ -f "$out/bin/hx" ]; then
                wrapProgram "$out/bin/hx" --set HELIX_RUNTIME "${runtimeDir}"
              fi
            '';
          };
        shell = common: prev: {
          packages = prev.packages ++ (with common.pkgs; [ lld_12 lldb cargo-tarpaulin ]);
          env = prev.env ++ [
            { name = "HELIX_RUNTIME"; eval = "$PWD/runtime"; }
            { name = "RUST_BACKTRACE"; value = "1"; }
            { name = "RUSTFLAGS"; value = "-C link-arg=-fuse-ld=lld -C target-cpu=native"; }
          ];
        };
      };
    };
}
