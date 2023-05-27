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
    parts.url = "github:hercules-ci/flake-parts";
  };

  outputs = inp: let
    mkRootPath = rel:
      builtins.path {
        path = "${toString ./.}/${rel}";
        name = rel;
      };
    filteredSource = let
      pathsToIgnore = [
        ".envrc"
        ".ignore"
        ".github"
        ".gitignore"
        "logo.svg"
        "logo_dark.svg"
        "logo_light.svg"
        "rust-toolchain.toml"
        "rustfmt.toml"
        "runtime"
        "screenshot.png"
        "book"
        "contrib"
        "docs"
        "README.md"
        "CHANGELOG.md"
        "shell.nix"
        "default.nix"
        "grammars.nix"
        "flake.nix"
        "flake.lock"
      ];
      ignorePaths = path: type: let
        inherit (inp.nixpkgs) lib;
        # split the nix store path into its components
        components = lib.splitString "/" path;
        # drop off the `/nix/hash-source` section from the path
        relPathComponents = lib.drop 4 components;
        # reassemble the path components
        relPath = lib.concatStringsSep "/" relPathComponents;
      in
        lib.all (p: ! (lib.hasPrefix p relPath)) pathsToIgnore;
    in
      builtins.path {
        name = "helix-source";
        path = toString ./.;
        # filter out unnecessary paths
        filter = ignorePaths;
      };
  in
    inp.parts.lib.mkFlake {inputs = inp;} {
      imports = [inp.nci.flakeModule inp.parts.flakeModules.easyOverlay];
      systems = [
        "x86_64-linux"
        "x86_64-darwin"
        "aarch64-linux"
        "aarch64-darwin"
        "i686-linux"
      ];
      perSystem = {
        config,
        pkgs,
        lib,
        ...
      }: let
        makeOverridableHelix = old: config: let
          grammars = pkgs.callPackage ./grammars.nix config;
          runtimeDir = pkgs.runCommand "helix-runtime" {} ''
            mkdir -p $out
            ln -s ${mkRootPath "runtime"}/* $out
            rm -r $out/grammars
            ln -s ${grammars} $out/grammars
          '';
          helix-wrapped =
            pkgs.runCommand
            old.name
            {
              inherit (old) pname version;
              meta = old.meta or {};
              passthru =
                (old.passthru or {})
                // {
                  unwrapped = old;
                };
              nativeBuildInputs = [pkgs.makeWrapper];
              makeWrapperArgs = config.makeWrapperArgs or [];
            }
            ''
              cp -rs --no-preserve=mode,ownership ${old} $out
              wrapProgram "$out/bin/hx" ''${makeWrapperArgs[@]} --set HELIX_RUNTIME "${runtimeDir}"
            '';
        in
          helix-wrapped
          // {
            override = makeOverridableHelix old;
            passthru =
              helix-wrapped.passthru
              // {
                wrapper = old: makeOverridableHelix old config;
              };
          };
        stdenv =
          if pkgs.stdenv.isLinux
          then pkgs.stdenv
          else pkgs.clangStdenv;
        rustFlagsEnv =
          if stdenv.isLinux
          then ''$RUSTFLAGS -C link-arg=-fuse-ld=lld -C target-cpu=native -Clink-arg=-Wl,--no-rosegment''
          else "$RUSTFLAGS";
      in {
        nci.projects."helix-project".relPath = "";
        nci.crates."helix-term" = {
          overrides = {
            add-meta.override = _: {meta.mainProgram = "hx";};
            add-inputs.overrideAttrs = prev: {
              buildInputs = (prev.buildInputs or []) ++ [stdenv.cc.cc.lib];
            };
            disable-grammar-builds = {
              # disable fetching and building of tree-sitter grammars in the helix-term build.rs
              HELIX_DISABLE_AUTO_GRAMMAR_BUILD = "1";
            };
            disable-tests = {checkPhase = ":";};
            set-stdenv.override = _: {inherit stdenv;};
            set-filtered-src.override = _: {src = filteredSource;};
          };
        };

        packages.helix-unwrapped = config.nci.outputs."helix-term".packages.release;
        packages.helix-unwrapped-dev = config.nci.outputs."helix-term".packages.dev;
        packages.helix = makeOverridableHelix config.packages.helix-unwrapped {};
        packages.helix-dev = makeOverridableHelix config.packages.helix-unwrapped-dev {};
        packages.default = config.packages.helix;

        overlayAttrs = {
          inherit (config.packages) helix;
        };

        devShells.default = config.nci.outputs."helix-project".devShell.overrideAttrs (old: {
          nativeBuildInputs =
            (old.nativeBuildInputs or [])
            ++ (with pkgs; [lld_13 cargo-flamegraph rust-analyzer])
            ++ (lib.optional (stdenv.isx86_64 && stdenv.isLinux) pkgs.cargo-tarpaulin)
            ++ (lib.optional stdenv.isLinux pkgs.lldb)
            ++ (lib.optional stdenv.isDarwin pkgs.darwin.apple_sdk.frameworks.CoreFoundation);
          shellHook = ''
            export HELIX_RUNTIME="$PWD/runtime"
            export RUST_BACKTRACE="1"
            export RUSTFLAGS="${rustFlagsEnv}"
          '';
        });
      };
    };

  nixConfig = {
    extra-substituters = ["https://helix.cachix.org"];
    extra-trusted-public-keys = ["helix.cachix.org-1:ejp9KQpR1FBI2onstMQ34yogDm4OgU2ru6lIwPvuCVs="];
  };
}
