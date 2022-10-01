{
  description = "A post-modern text editor.";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nci = {
      url = "github:yusdacra/nix-cargo-integration";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-overlay.follows = "rust-overlay";
    };
  };

  outputs = {
    self,
    nixpkgs,
    nci,
    ...
  }: let
    lib = nixpkgs.lib;
    mkRootPath = rel:
      builtins.path {
        path = "${toString ./.}/${rel}";
        name = rel;
      };
    outputs = nci.lib.makeOutputs {
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
          helix-term = prev: {
            src = builtins.path {
              name = "helix-source";
              path = toString ./.;
              # filter out unneeded stuff that cause rebuilds
              filter = path: type:
                lib.all
                (n: builtins.baseNameOf path != n)
                [
                  ".envrc"
                  ".ignore"
                  ".github"
                  "runtime"
                  "screenshot.png"
                  "book"
                  "contrib"
                  "docs"
                  "README.md"
                  "shell.nix"
                  "default.nix"
                  "grammars.nix"
                  "flake.nix"
                  "flake.lock"
                ];
            };

            # disable fetching and building of tree-sitter grammars in the helix-term build.rs
            HELIX_DISABLE_AUTO_GRAMMAR_BUILD = "1";

            buildInputs = (prev.buildInputs or []) ++ [common.cCompiler.cc.lib];

            # link languages and theme toml files since helix-term expects them (for tests)
            preConfigure = ''
              ${prev.preConfigure or ""}
              ${
                lib.concatMapStringsSep
                "\n"
                (path: "ln -sf ${mkRootPath path} ..")
                ["languages.toml" "theme.toml" "base16_theme.toml"]
              }
            '';

            meta.mainProgram = "hx";
          };
        };
        shell = common: prev: {
          packages =
            prev.packages
            ++ (
              with common.pkgs;
                [lld_13 cargo-flamegraph rust-analyzer]
                ++ (lib.optional (stdenv.isx86_64 && stdenv.isLinux) cargo-tarpaulin)
                ++ (lib.optional stdenv.isLinux lldb)
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
    makeOverridableHelix = system: old: config: let
      pkgs = nixpkgs.legacyPackages.${system};
      grammars = pkgs.callPackage ./grammars.nix config;
      runtimeDir = pkgs.runCommand "helix-runtime" {} ''
        mkdir -p $out
        ln -s ${mkRootPath "runtime"}/* $out
        rm -r $out/grammars
        ln -s ${grammars} $out/grammars
      '';
      helix-wrapped =
        pkgs.runCommand "${old.name}-wrapped"
        {
          inherit (old) pname version meta;

          nativeBuildInputs = [pkgs.makeWrapper];
          makeWrapperArgs = config.makeWrapperArgs or [];
        }
        ''
          mkdir -p $out
          cp -r --no-preserve=mode,ownership ${old}/* $out/
          chmod +x $out/bin/*
          wrapProgram "$out/bin/hx" ''${makeWrapperArgs[@]} --set HELIX_RUNTIME "${runtimeDir}"
        '';
    in
      helix-wrapped
      // {override = makeOverridableHelix system old;};
  in
    outputs
    // {
      apps =
        lib.mapAttrs
        (
          system: apps: rec {
            default = hx;
            hx = {
              type = "app";
              program = lib.getExe self.${system}.packages.helix;
            };
          }
        )
        outputs.apps;
      packages =
        lib.mapAttrs
        (
          system: packages: rec {
            default = helix;
            helix = makeOverridableHelix system helix-unwrapped {};
            helix-debug = makeOverridableHelix system helix-unwrapped-debug {};
            helix-unwrapped = packages.helix;
            helix-unwrapped-debug = packages.helix-debug;
          }
        )
        outputs.packages;
    };

  nixConfig = {
    extra-substituters = ["https://helix.cachix.org"];
    extra-trusted-public-keys = ["helix.cachix.org-1:ejp9KQpR1FBI2onstMQ34yogDm4OgU2ru6lIwPvuCVs="];
  };
}
