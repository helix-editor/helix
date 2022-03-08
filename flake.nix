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
      inputs.rustOverlay.follows = "rust-overlay";
    };
    # NOTE: the flake looks like it is hanging when it pulls this input because
    # the submodules take a long time to clone. This will be fixed in #1659.
    helix = {
      url = "https://github.com/helix-editor/helix.git";
      type = "git";
      submodules = true;
      flake = false;
    };
  };

  outputs = inputs@{ nixCargoIntegration, helix, ... }:
    nixCargoIntegration.lib.makeOutputs {
      root = ./.;
      renameOutputs = { "helix-term" = "helix"; };
      # Set default app to hx (binary is from helix-term release build)
      # Set default package to helix-term release build
      defaultOutputs = {
        app = "hx";
        package = "helix";
      };
      overrides = {
        crateOverrides = common: _: rec {
          helix-term = prev: {
            buildInputs = (prev.buildInputs or [ ]) ++ [ common.cCompiler.cc.lib ];
            nativeBuildInputs = (prev.nativeBuildInputs or [ ]) ++ [ common.pkgs.makeWrapper ];
            preConfigure = ''
              ${prev.preConfigure}
              rm -r helix-syntax/languages
              ln -s ${helix}/helix-syntax/languages helix-syntax/languages
              ln -s "$PWD/helix-syntax/languages" languages
              mkdir -p runtime/grammars
            '';
            postInstall = ''
              ${prev.postInstall or ""}
              mkdir -p $out/lib
              cp -r runtime $out/lib
              wrapProgram "$out/bin/hx" --set HELIX_RUNTIME "$out/lib/runtime"
            '';
          };
        };
        shell = common: prev: {
          packages = prev.packages ++ (with common.pkgs; [ lld_13 lldb cargo-tarpaulin cargo-flamegraph ]);
          env = prev.env ++ [
            { name = "HELIX_RUNTIME"; eval = "$PWD/runtime"; }
            { name = "RUST_BACKTRACE"; value = "1"; }
            { name = "RUSTFLAGS"; value = "-C link-arg=-fuse-ld=lld -C target-cpu=native -Clink-arg=-Wl,--no-rosegment"; }
          ];
        };
      };
    };
}
