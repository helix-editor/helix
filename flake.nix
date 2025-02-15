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
    mkHelix = {
      pkgs,
      rustPlatform,
      stdenv,
      ...
    }: let
      # Next we actually need to build the grammars and the runtime directory
      # that they reside in. It is built by calling the derivation in the
      # grammars.nix file, then taking the runtime directory in the git repo
      # and hooking symlinks up to it.
      grammars = pkgs.callPackage ./grammars.nix {};
      runtimeDir = pkgs.runCommand "helix-runtime" {} ''
        mkdir -p $out
        ln -s ${./runtime}/* $out
        rm -r $out/grammars
        ln -s ${grammars} $out/grammars
      '';
    in
      # Currently rustPlatform.buildRustPackage doesn't have the finalAttrs pattern
      # hooked up. To get around this while having good customization, mkDerivation is
      # used instead.
      stdenv.mkDerivation (self: {
        # START: Reevaluate the below attrs when
        # https://github.com/NixOS/nixpkgs/pull/354999
        # or
        # https://github.com/NixOS/nixpkgs/pull/194475
        # Are merged.

        # TODO: Probably change to cargoLock
        cargoDeps = rustPlatform.importCargoLock {
          lockFile = ./Cargo.lock;
        };

        nativeBuildInputs = [
          rustPlatform.rust.rustc # TODO: Remove
          rustPlatform.rust.cargo # TODO: Remove
          pkgs.installShellFiles
          pkgs.git
        ];

        # TODO: Remove entire attr
        buildInputs = with rustPlatform; [
          cargoSetupHook
          cargoBuildHook
          cargoInstallHook
        ];

        # Use Helix's opt profile for the build.
        # TODO: s/cargoBuildType/buildType
        cargoBuildType = "opt";
        # END: Funny attrs to reevaluate

        name = with builtins; (fromTOML (readFile ./helix-term/Cargo.toml)).package.name;
        src = pkgs.lib.sources.cleanSource ./.;

        # Helix attempts to reach out to the network and get the grammars. Nix doesn't allow this.
        HELIX_DISABLE_AUTO_GRAMMAR_BUILD = "1";

        # So Helix knows what rev it is.
        HELIX_NIX_BUILD_REV = self.rev or self.dirtyRev or null;

        doCheck = false;
        strictDeps = true;

        # Sets the Helix runtimedir to the grammars
        env.HELIX_DEFAULT_RUNTIME = "${runtimeDir}";

        # Get all the application stuff in the output directory.
        postInstall = ''
          mkdir -p $out/lib
          installShellCompletion ${./contrib/completion}/hx.{bash,fish,zsh}
          mkdir -p $out/share/{applications,icons/hicolor/256x256/apps}
          cp ${./contrib/Helix.desktop} $out/share/applications
          cp ${./contrib/helix.png} $out/share/icons/hicolor/256x256/apps
        '';

        meta.mainProgram = "hx";
      });
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
      packages = {
        # Make MSRV Helix
        helix = pkgs.callPackage mkHelix {rustPlatform = msrvPlatform;};

        # The default Helix build. Uses the default MSRV Rust toolchain, and the
        # default nixpkgs, which is the one in the Flake.lock of Helix.
        #
        # This can be overridden though to add Cargo Features, flags, and different toolchains.
        default = self.packages.${system}.helix;
      };

      checks.helix = self.outputs.packages.${system}.helix.overrideAttrs {
        cargoBuildType = "debug";
      };

      formatter = pkgs.alejandra;

      # Devshell behavior is preserved.
      devShells.default = let
        rustFlagsEnv = pkgs.lib.optionalString pkgs.stdenv.isLinux "-C link-arg=-fuse-ld=lld -C target-cpu=native -Clink-arg=-Wl,--no-rosegment --cfg tokio_unstable";
      in
        pkgs.mkShell
        {
          inputsFrom = builtins.attrValues self.checks.${system};
          nativeBuildInputs = with pkgs;
            [lld_13 cargo-flamegraph rust-analyzer]
            ++ (lib.optional (stdenv.isx86_64 && stdenv.isLinux) pkgs.cargo-tarpaulin)
            ++ (lib.optional stdenv.isLinux pkgs.lldb)
            ++ (lib.optional stdenv.isDarwin pkgs.darwin.apple_sdk.frameworks.CoreFoundation);
          shellHook = ''
            export HELIX_RUNTIME="$PWD/runtime"
            export RUST_BACKTRACE="1"
            export RUSTFLAGS="''${RUSTFLAGS:-""} ${rustFlagsEnv}"
          '';
        };
    })
    // {
      overlays.default = final: prev: {
        helix = final.callPackage mkHelix {};
      };
    };

  nixConfig = {
    extra-substituters = ["https://helix.cachix.org"];
    extra-trusted-public-keys = ["helix.cachix.org-1:ejp9KQpR1FBI2onstMQ34yogDm4OgU2ru6lIwPvuCVs="];
  };
}
