{
  description = "A post-modern text editor.";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixCargoIntegration = {
      url = "github:yusdacra/nix-cargo-integration";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-overlay.follows = "rust-overlay";
    };
  };

  outputs = inputs @ {
    nixpkgs,
    nixCargoIntegration,
    ...
  }: let
    outputs = config:
      nixCargoIntegration.lib.makeOutputs {
        root = ./.;
        renameOutputs = {"helix-term" = "helix";};
        # Set default app to hx (binary is from helix-term release build)
        # Set default package to helix-term release build
        defaultOutputs = {
          app = "hx";
          package = "helix";
        };
        overrides = {
          cCompiler = common:
            with common.pkgs;
              if stdenv.isLinux
              then gcc
              else clang;
          crateOverrides = common: _: {
            helix-term = prev: let
              inherit (common) pkgs;
              mkRootPath = rel:
                builtins.path {
                  path = "${common.root}/${rel}";
                  name = rel;
                };
              grammars = pkgs.callPackage ./grammars.nix config;
              runtimeDir = pkgs.runCommandNoCC "helix-runtime" {} ''
                mkdir -p $out
                ln -s ${mkRootPath "runtime"}/* $out
                rm -r $out/grammars
                ln -s ${grammars} $out/grammars
              '';
              overridedAttrs = {
                # disable fetching and building of tree-sitter grammars in the helix-term build.rs
                HELIX_DISABLE_AUTO_GRAMMAR_BUILD = "1";
                # link languages and theme toml files since helix-term expects them (for tests)
                preConfigure =
                  pkgs.lib.concatMapStringsSep
                  "\n"
                  (path: "ln -sf ${mkRootPath path} ..")
                  ["languages.toml" "theme.toml" "base16_theme.toml"];
                buildInputs = (prev.buildInputs or []) ++ [common.cCompiler.cc.lib];
                nativeBuildInputs = [pkgs.makeWrapper];

                postFixup = ''
                  if [ -f "$out/bin/hx" ]; then
                    wrapProgram "$out/bin/hx" ''${makeWrapperArgs[@]} --set HELIX_RUNTIME "${runtimeDir}"
                  fi
                '';
              };
            in
              overridedAttrs
              // (
                pkgs.lib.optionalAttrs
                (config ? makeWrapperArgs)
                {inherit (config) makeWrapperArgs;}
              );
          };
          shell = common: prev: {
            packages =
              prev.packages
              ++ (
                with common.pkgs;
                [lld_13 lldb cargo-flamegraph rust-analyzer] ++
                (lib.optional (stdenv.isx86_64 && stdenv.isLinux) cargo-tarpaulin)
              );
            env =
              prev.env
              ++ [
                {
                  name = "HELIX_RUNTIME";
                  eval = "$PWD/runtime";
                }
                {
                  name = "RUST_BACKTRACE";
                  value = "1";
                }
                {
                  name = "RUSTFLAGS";
                  value =
                    if common.pkgs.stdenv.isLinux
                    then "-C link-arg=-fuse-ld=lld -C target-cpu=native -Clink-arg=-Wl,--no-rosegment"
                    else "";
                }
              ];
          };
        };
      };
    defaultOutputs = outputs {};
    makeOverridableHelix = system: old:
      old
      // {
        override = args:
          makeOverridableHelix
          system
          (outputs args).packages.${system}.helix;
      };
  in
    defaultOutputs
    // {
      packages =
        nixpkgs.lib.mapAttrs
        (
          system: packages:
            packages
            // rec {
              default = helix;
              helix = makeOverridableHelix system packages.helix;
            }
        )
        defaultOutputs.packages;
    };

  nixConfig = {
    extra-substituters = ["https://helix.cachix.org"];
    extra-trusted-public-keys = ["helix.cachix.org-1:ejp9KQpR1FBI2onstMQ34yogDm4OgU2ru6lIwPvuCVs="];
  };
}
