{
  lib,
  rustPlatform,
  callPackage,
  runCommand,
  installShellFiles,
  git,
  ...
}: let
  fs = lib.fileset;

  src = fs.difference (fs.gitTracked ./.) (fs.unions [
    ./.envrc
    ./rustfmt.toml
    ./screenshot.png
    ./book
    ./docs
    ./flake.lock
    (fs.fileFilter (file: lib.strings.hasInfix ".git" file.name) ./.)
    (fs.fileFilter (file: file.hasExt "svg") ./.)
    (fs.fileFilter (file: file.hasExt "md") ./.)
    (fs.fileFilter (file: file.hasExt "nix") ./.)
  ]);

  # Next we actually need to build the grammars and the runtime directory
  # that they reside in. It is built by calling the derivation in the
  # grammars.nix file, then taking the runtime directory in the git repo
  # and hooking symlinks up to it.
  grammars = callPackage ./grammars.nix {};
  runtimeDir = runCommand "helix-runtime" {} ''
    mkdir -p $out
    ln -s ${./runtime}/* $out
    rm -r $out/grammars
    ln -s ${grammars} $out/grammars
  '';
in
  # Currently rustPlatform.buildRustPackage doesn't have the finalAttrs pattern
  # hooked up. To get around this while having good customization, mkDerivation is
  # used instead.
  rustPlatform.buildRustPackage (self: {
    cargoLock.lockFile = ./Cargo.lock;

    nativeBuildInputs = [
      installShellFiles
      git
    ];

    buildType = "release";

    name = with builtins; (fromTOML (readFile ./helix-term/Cargo.toml)).package.name;
    src = fs.toSource {
      root = ./.;
      fileset = src;
    };

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
  })
