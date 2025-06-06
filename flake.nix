# {
#   description = "A post-modern text editor.";

#   inputs = {
#     nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
#     rust-overlay = {
#       url = "github:oxalica/rust-overlay";
#       inputs.nixpkgs.follows = "nixpkgs";
#     };
#     crane.url = "github:ipetkov/crane";
#   };

#   outputs = {
#     self,
#     nixpkgs,
# <<<<<<< HEAD
#     crane,
#     flake-utils,
#     rust-overlay,
#     ...
#   }:
#     flake-utils.lib.eachDefaultSystem (system: let
#       pkgs = import nixpkgs {
#         inherit system;
#         overlays = [(import rust-overlay)];
#       };
#       mkRootPath = rel:
#         builtins.path {
#           path = "${toString ./.}/${rel}";
#           name = rel;
#         };
#       filteredSource = let
#         pathsToIgnore = [
#           ".envrc"
#           ".ignore"
#           ".github"
#           ".gitignore"
#           "logo_dark.svg"
#           "logo_light.svg"
#           "rust-toolchain.toml"
#           "rustfmt.toml"
#           "runtime"
#           "screenshot.png"
#           "book"
#           "docs"
#           "README.md"
#           "CHANGELOG.md"
#           "shell.nix"
#           "default.nix"
#           "grammars.nix"
#           "flake.nix"
#           "flake.lock"
#         ];
#         ignorePaths = path: type: let
#           inherit (nixpkgs) lib;
#           # split the nix store path into its components
#           components = lib.splitString "/" path;
#           # drop off the `/nix/hash-source` section from the path
#           relPathComponents = lib.drop 4 components;
#           # reassemble the path components
#           relPath = lib.concatStringsSep "/" relPathComponents;
#         in
#           lib.all (p: ! (lib.hasPrefix p relPath)) pathsToIgnore;
#       in
#         builtins.path {
#           name = "helix-source";
#           path = toString ./.;
#           # filter out unnecessary paths
#           filter = ignorePaths;
#         };


#       helix-cogs = craneLibStable.buildPackage (commonArgs // {
#         pname = "helix-cogs";
#         version = "0.1.0";
#         # cargoArtifacts = craneLibStable.buildDepsOnly commonArgs;

#         buildPhase = ''
#           export HOME=$PWD/build_home  # code-gen will write files relative to $HOME
#           mkdir -p $HOME
#           cargoBuildLog=$(mktemp cargoBuildLogXXXX.json)
#           cargo run --package xtask -- code-gen >"$cargoBuildLog"
#         '';

#         installPhase = ''
#           mkdir -p $out/cogs
#           cp -r build_home/.steel/cogs/helix/* "$out/cogs"
#         '';

#       });

#       makeOverridableHelix = old: config: let
#         grammars = pkgs.callPackage ./grammars.nix config;
#         runtimeDir = pkgs.runCommand "helix-runtime" {} ''
#           mkdir -p $out
#           ln -s ${mkRootPath "runtime"}/* $out
#           rm -r $out/grammars
#           ln -s ${grammars} $out/grammars
#         '';
#         helix-wrapped =
#           pkgs.runCommand
#           old.name
#           {
#             inherit (old) pname version;
#             meta = old.meta or {};
#             passthru =
#               (old.passthru or {})
#               // {
#                 unwrapped = old;
#               };
#             nativeBuildInputs = [pkgs.makeWrapper];
#             makeWrapperArgs = config.makeWrapperArgs or [];
#           }
#           ''
#             cp -rs --no-preserve=mode,ownership ${old} $out
#             wrapProgram "$out/bin/hx" ''${makeWrapperArgs[@]} --set HELIX_RUNTIME "${runtimeDir}"
#           '';
#       in
#         helix-wrapped
#         // {
#           override = makeOverridableHelix old;
#           passthru =
#             helix-wrapped.passthru
#             // {
#               wrapper = old: makeOverridableHelix old config;
#             };
#         };
#       stdenvSelector = p:
#         if p.stdenv.isLinux
#         then p.stdenv
#         else p.clangStdenv;
#       stdenv = stdenvSelector pkgs;
#       rustFlagsEnv = pkgs.lib.optionalString stdenv.isLinux "-C link-arg=-fuse-ld=lld -C target-cpu=native -Clink-arg=-Wl,--no-rosegment --cfg tokio_unstable";
#       rustToolchain = pkgs.pkgsBuildHost.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
#       craneLibMSRV = (crane.mkLib pkgs).overrideToolchain rustToolchain;
#       craneLibStable = (crane.mkLib pkgs).overrideToolchain pkgs.pkgsBuildHost.rust-bin.stable.latest.default;
#       commonArgs = {
#         stdenv = stdenvSelector;
#         inherit (craneLibMSRV.crateNameFromCargoToml {cargoToml = ./helix-term/Cargo.toml;}) pname;
#         inherit (craneLibMSRV.crateNameFromCargoToml {cargoToml = ./Cargo.toml;}) version;
#         src = filteredSource;
#         # disable fetching and building of tree-sitter grammars in the helix-term build.rs
#         HELIX_DISABLE_AUTO_GRAMMAR_BUILD = "1";
#         buildInputs = [stdenv.cc.cc.lib];
#         nativeBuildInputs = [pkgs.installShellFiles];
#         # disable tests
#         doCheck = false;
#         meta.mainProgram = "hx";
#       };
#       cargoArtifacts = craneLibMSRV.buildDepsOnly commonArgs;
#     in {
#       packages = {
#         helix-unwrapped = craneLibStable.buildPackage (commonArgs
#           // {
#             cargoArtifacts = craneLibStable.buildDepsOnly commonArgs;
#             postInstall = ''
#               mkdir -p $out/share/applications $out/share/icons/hicolor/scalable/apps $out/share/icons/hicolor/256x256/apps
#               cp contrib/Helix.desktop $out/share/applications
#               cp logo.svg $out/share/icons/hicolor/scalable/apps/helix.svg
#               cp contrib/helix.png $out/share/icons/hicolor/256x256/apps
#               installShellCompletion contrib/completion/hx.{bash,fish,zsh}
#             '';
#             # set git revision for nix flake builds, see 'git_hash' in helix-loader/build.rs
#             HELIX_NIX_BUILD_REV = self.rev or self.dirtyRev or null;
#           });
#         helix = makeOverridableHelix self.packages.${system}.helix-unwrapped {};
#         helix-cogs = helix-cogs;
#         default = self.packages.${system}.helix;
#       };

#       checks = {
#         # Build the crate itself
#         inherit (self.packages.${system}) helix;

#         clippy = craneLibMSRV.cargoClippy (commonArgs
#           // {
#             inherit cargoArtifacts;
#             cargoClippyExtraArgs = "--all-targets -- --deny warnings";
#           });

#         fmt = craneLibMSRV.cargoFmt commonArgs;

#         doc = craneLibMSRV.cargoDoc (commonArgs
#           // {
#             inherit cargoArtifacts;
#           });

#         test = craneLibMSRV.cargoTest (commonArgs
#           // {
#             inherit cargoArtifacts;
#           });
#       };

#       devShells.default = pkgs.mkShell {
#         inputsFrom = builtins.attrValues self.checks.${system};
#         nativeBuildInputs = with pkgs;
#           [lld_13 cargo-flamegraph rust-analyzer]
#           ++ (lib.optional (stdenv.isx86_64 && stdenv.isLinux) pkgs.cargo-tarpaulin)
#           ++ (lib.optional stdenv.isLinux pkgs.lldb)
#           ++ (lib.optional stdenv.isDarwin (with pkgs.darwin.apple_sdk.frameworks;
#             [CoreFoundation Security]));
#         shellHook = ''
#           export HELIX_RUNTIME="$PWD/runtime"
#           export RUST_BACKTRACE="1"
#           export RUSTFLAGS="''${RUSTFLAGS:-""} ${rustFlagsEnv}"
#         '';
#       };
#     })
#     // {
#       overlays.default = final: prev: {
#         inherit (self.packages.${final.system}) helix;
# =======
#     rust-overlay,
#     ...
#   }: let
#     inherit (nixpkgs) lib;
#     systems = [
#       "x86_64-linux"
#       "aarch64-linux"
#       "x86_64-darwin"
#       "aarch64-darwin"
#     ];
#     eachSystem = lib.genAttrs systems;
#     pkgsFor = eachSystem (system:
#       import nixpkgs {
#         localSystem.system = system;
#         overlays = [(import rust-overlay) self.overlays.helix];
#       });
#     gitRev = self.rev or self.dirtyRev or null;
#   in {
#     packages = eachSystem (system: {
#       inherit (pkgsFor.${system}) helix;
#       /*
#       The default Helix build. Uses the latest stable Rust toolchain, and unstable
#       nixpkgs.

#       The build inputs can be overridden with the following:

#       packages.${system}.default.override { rustPlatform = newPlatform; };

#       Overriding a derivation attribute can be done as well:

#       packages.${system}.default.overrideAttrs { buildType = "debug"; };
#       */
#       default = self.packages.${system}.helix;
#     });
#     checks =
#       lib.mapAttrs (system: pkgs: let
#         # Get Helix's MSRV toolchain to build with by default.
#         msrvToolchain = pkgs.pkgsBuildHost.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
#         msrvPlatform = pkgs.makeRustPlatform {
#           cargo = msrvToolchain;
#           rustc = msrvToolchain;
#         };
#       in {
#         helix = self.packages.${system}.helix.override {
#           rustPlatform = msrvPlatform;
#         };
#       })
#       pkgsFor;

#     # Devshell behavior is preserved.
#     devShells =
#       lib.mapAttrs (system: pkgs: {
#         default = let
#           commonRustFlagsEnv = "-C link-arg=-fuse-ld=lld -C target-cpu=native --cfg tokio_unstable";
#           platformRustFlagsEnv = lib.optionalString pkgs.stdenv.isLinux "-Clink-arg=-Wl,--no-rosegment";
#         in
#           pkgs.mkShell {
#             inputsFrom = [self.checks.${system}.helix];
#             nativeBuildInputs = with pkgs;
#               [
#                 lld
#                 cargo-flamegraph
#                 rust-bin.nightly.latest.rust-analyzer
#               ]
#               ++ (lib.optional (stdenv.isx86_64 && stdenv.isLinux) cargo-tarpaulin)
#               ++ (lib.optional stdenv.isLinux lldb)
#               ++ (lib.optional stdenv.isDarwin darwin.apple_sdk.frameworks.CoreFoundation);
#             shellHook = ''
#               export RUST_BACKTRACE="1"
#               export RUSTFLAGS="''${RUSTFLAGS:-""} ${commonRustFlagsEnv} ${platformRustFlagsEnv}"
#             '';
#           };
#       })
#       pkgsFor;

#     overlays = {
#       helix = final: prev: {
#         helix = final.callPackage ./default.nix {inherit gitRev;};
# >>>>>>> origin
#       };

#       default = self.overlays.helix;
#     };
#   };
#   nixConfig = {
#     extra-substituters = ["https://helix.cachix.org"];
#     extra-trusted-public-keys = ["helix.cachix.org-1:ejp9KQpR1FBI2onstMQ34yogDm4OgU2ru6lIwPvuCVs="];
#   };
# }

{
  description = "A post-modern text editor.";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    ...
  }: let
    inherit (nixpkgs) lib;
    systems = [
      "x86_64-linux"
      "aarch64-linux"
      "x86_64-darwin"
      "aarch64-darwin"
    ];
    eachSystem = lib.genAttrs systems;
    pkgsFor = eachSystem (system:
      import nixpkgs {
        localSystem.system = system;
        overlays = [(import rust-overlay) self.overlays.helix];
      });
    gitRev = self.rev or self.dirtyRev or null;
  in {
    packages = eachSystem (system: {
      inherit (pkgsFor.${system}) helix;
      /*
      The default Helix build. Uses the latest stable Rust toolchain, and unstable
      nixpkgs.

      The build inputs can be overridden with the following:

      packages.${system}.default.override { rustPlatform = newPlatform; };

      Overriding a derivation attribute can be done as well:

      packages.${system}.default.overrideAttrs { buildType = "debug"; };
      */
      default = self.packages.${system}.helix;
    });
    checks =
      lib.mapAttrs (system: pkgs: let
        # Get Helix's MSRV toolchain to build with by default.
        msrvToolchain = pkgs.pkgsBuildHost.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        msrvPlatform = pkgs.makeRustPlatform {
          cargo = msrvToolchain;
          rustc = msrvToolchain;
        };
      in {
        helix = self.packages.${system}.helix.override {
          rustPlatform = msrvPlatform;
        };
      })
      pkgsFor;

    # Devshell behavior is preserved.
    devShells =
      lib.mapAttrs (system: pkgs: {
        default = let
          commonRustFlagsEnv = "-C link-arg=-fuse-ld=lld -C target-cpu=native --cfg tokio_unstable";
          platformRustFlagsEnv = lib.optionalString pkgs.stdenv.isLinux "-Clink-arg=-Wl,--no-rosegment";
        in
          pkgs.mkShell {
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
      pkgsFor;

    overlays = {
      helix = final: prev: {
        helix = final.callPackage ./default.nix {inherit gitRev;};
      };

      default = self.overlays.helix;
    };
  };
  nixConfig = {
    extra-substituters = ["https://helix.cachix.org"];
    extra-trusted-public-keys = ["helix.cachix.org-1:ejp9KQpR1FBI2onstMQ34yogDm4OgU2ru6lIwPvuCVs="];
  };
}
