{
  description = "A post-modern text editor.";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
  };

  outputs = {
    self,
    nixpkgs,
    crane,
    flake-utils,
    rust-overlay,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      # Apply the rust-overlay to nixpkgs so we can access all version.
      pkgs = import nixpkgs {
        inherit system;
        overlays = [(import rust-overlay)];
      };

      # Get Helix's MSRV toolchain to build with by default.
      rustToolchain = pkgs.pkgsBuildHost.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      craneLibMSRV = (crane.mkLib pkgs).overrideToolchain rustToolchain;

      # Common args for most things
      commonArgs = {
        # Helix attempts to reach out to the network and get the grammars. Nix doesn't allow this.
        HELIX_DISABLE_AUTO_GRAMMAR_BUILD = "1";

        # Get the name and version from the cargo.toml
        inherit (craneLibMSRV.crateNameFromCargoToml {cargoToml = ./helix-term/Cargo.toml;}) pname;
        inherit (craneLibMSRV.crateNameFromCargoToml {cargoToml = ./Cargo.toml;}) version;

        # Clean the source.
        src = craneLibMSRV.cleanCargoSource ./.;

        # Common build inputs.
        nativeBuildInputs = [pkgs.installShellFiles];

        # disable tests
        doCheck = false;
        strictDeps = true;
      };

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

      # Cranelib allows us to put the dependencies in the nix store. This means
      # it is semi-incremental if the Cargo.lock doesn't change.
      cargoArtifacts = craneLibMSRV.buildDepsOnly commonArgs;

      # This allows for an overridable helix build.
      #
      build_helix = pkgs.lib.makeOverridable ({
        pkgs,
        craneLib,
        runtimeDir,
        cargoExtraArgs ? "",
      }:
        craneLib.buildPackage (commonArgs
          // rec {
            inherit cargoArtifacts cargoExtraArgs;
            nativeBuildInputs = [
              pkgs.installShellFiles
              pkgs.git
            ];
            env.HELIX_DEFAULT_RUNTIME = "${runtimeDir}";

            postInstall = ''
              mkdir -p $out/lib
              installShellCompletion contrib/completion/hx.{bash,fish,zsh}
              mkdir -p $out/share/{applications,icons/hicolor/256x256/apps}
              cp contrib/Helix.desktop $out/share/applications
              cp contrib/helix.png $out/share/icons/hicolor/256x256/apps
            '';

            meta = {
              mainProgram = "hx";
            };
          }));
    in {
      packages = {
        helix = build_helix {
          inherit pkgs runtimeDir;
          craneLib = craneLibMSRV;
        };

        # The default Helix build. Uses the default MSRV Rust toolchain, and the
        # default nixpkgs, which is the one in the Flake.lock of Helix.
        #
        # This can be overridden though to add Cargo Features, flags, and different toolchains.
        default = self.packages.${system}.helix;
      };

      checks = {
        # Build the crate itself
        inherit (self.packages.${system}) helix;

        clippy = craneLibMSRV.cargoClippy (commonArgs
          // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });

        fmt = craneLibMSRV.cargoFmt commonArgs;

        doc = craneLibMSRV.cargoDoc (commonArgs
          // {
            inherit cargoArtifacts;
          });

        test = craneLibMSRV.cargoTest (commonArgs
          // {
            inherit cargoArtifacts;
          });
      };

      formatter = pkgs.alejandra;

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
        inherit (self.packages.${final.system}) helix;
      };
    };

  nixConfig = {
    extra-substituters = ["https://helix.cachix.org"];
    extra-trusted-public-keys = ["helix.cachix.org-1:ejp9KQpR1FBI2onstMQ34yogDm4OgU2ru6lIwPvuCVs="];
  };
}
