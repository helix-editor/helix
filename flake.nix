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
    ncl = nci.lib.nci-lib;
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
    outputs = nci.lib.makeOutputs {
      root = ./.;
      config = common: {
        outputs = {
          # rename helix-term to helix since it's our main package
          rename = {"helix-term" = "helix";};
          # Set default app to hx (binary is from helix-term release build)
          # Set default package to helix-term release build
          defaults = {
            app = "hx";
            package = "helix";
          };
        };
        cCompiler.package = with common.pkgs;
          if stdenv.isLinux
          then gcc
          else clang;
        shell = {
          packages = with common.pkgs;
            [lld_13 cargo-flamegraph rust-analyzer]
            ++ (lib.optional (stdenv.isx86_64 && stdenv.isLinux) cargo-tarpaulin)
            ++ (lib.optional stdenv.isLinux lldb)
            ++ (lib.optional stdenv.isDarwin darwin.apple_sdk.frameworks.CoreFoundation);
          env = [
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
              eval =
                if common.pkgs.stdenv.isLinux
                then "$RUSTFLAGS\" -C link-arg=-fuse-ld=lld -C target-cpu=native -Clink-arg=-Wl,--no-rosegment\""
                else "$RUSTFLAGS";
            }
          ];
        };
      };
      pkgConfig = common: {
        helix-term = let
          # Wrap helix with runtime
          wrapper = _: old: let
            inherit (common) pkgs;
            makeOverridableHelix = old: config: let
              grammars = pkgs.callPackage ./grammars.nix config;
              runtimeDir = pkgs.runCommand "helix-runtime" {} ''
                mkdir -p $out
                ln -s ${mkRootPath "runtime"}/* $out
                rm -r $out/grammars
                ln -s ${grammars} $out/grammars
              '';
              helix-wrapped =
                common.internal.pkgsSet.utils.wrapDerivation old
                {
                  nativeBuildInputs = [pkgs.makeWrapper];
                  makeWrapperArgs = config.makeWrapperArgs or [];
                }
                ''
                  rm -rf $out/bin
                  mkdir -p $out/bin
                  ln -sf ${old}/bin/* $out/bin/
                  wrapProgram "$out/bin/hx" ''${makeWrapperArgs[@]} --set HELIX_RUNTIME "${runtimeDir}"
                '';
            in
              helix-wrapped
              // {
                override = makeOverridableHelix old;
                passthru = helix-wrapped.passthru // {wrapper = wrapper {};};
              };
          in
            makeOverridableHelix old {};
        in {
          inherit wrapper;
          overrides.fix-build.overrideAttrs = prev: {
            src = filteredSource;

            # disable fetching and building of tree-sitter grammars in the helix-term build.rs
            HELIX_DISABLE_AUTO_GRAMMAR_BUILD = "1";

            buildInputs = ncl.addBuildInputs prev [common.config.cCompiler.package.cc.lib];

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
            checkPhase = ":";

            meta.mainProgram = "hx";
          };
        };
      };
    };
  in
    outputs
    // {
      packages =
        lib.mapAttrs
        (
          system: packages:
            packages
            // {
              helix-unwrapped = packages.helix.passthru.unwrapped;
              helix-unwrapped-dev = packages.helix-dev.passthru.unwrapped;
            }
        )
        outputs.packages;
    };

  nixConfig = {
    extra-substituters = ["https://helix.cachix.org"];
    extra-trusted-public-keys = ["helix.cachix.org-1:ejp9KQpR1FBI2onstMQ34yogDm4OgU2ru6lIwPvuCVs="];
  };
}
