# Helix flake package
#
# `grammarOverlays` can be used to override the normal helix `langauges.toml`
# grammars with those from nixpkgs if that behavior is desired using the
# following snippet as an argument to the `.override` of this package. There is
# one extra setting, and that is the `passthru.SHARED_LIB` for the tree-sitter
# grammar. If the tree sitter parser is built and is not in the file `shared`,
# then this can be set to the required path in the root of the package. Being
# that it is a `passthru` variable, it will not require a rebuild of the
# overridden package should that path change.
#
# Note: This may cause incompatability overriding all of the grammars like this.
#       Be sure that the nixpkgs grammar works with the version of helix being
#       built here!
#
# ```nix
# grammarOverlays = [
#   (final: prev:
#   builtins.mapAttrs (k: v:
#       pkgs.tree-sitter-grammars."tree-sitter-${k}" or v
#   ) prev)
# ];
# ```
{
  lib,
  rustPlatform,
  callPackage,
  runCommand,
  installShellFiles,
  git,
  makeWrapper,
  symlinkJoin,
  formats,
  gitRev ? null,
  grammarOverlays ? [ ],
  includeGrammarIf ? _: true,
}:
let
  fs = lib.fileset;
  readTOML = f: with builtins; fromTOML (readFile f);
  cargoTOML = readTOML ./helix-term/Cargo.toml;
  wsTOML = readTOML ./Cargo.toml;

  src = fs.difference (fs.gitTracked ./.) (
    fs.unions [
      ./.envrc
      ./rustfmt.toml
      ./screenshot.png
      ./book
      ./docs
      ./runtime
      ./flake.lock
      ./languages.toml
      ./runtime/grammars
      (fs.fileFilter (file: lib.strings.hasInfix ".git" file.name) ./.)
      (fs.fileFilter (file: file.hasExt "svg") ./.)
      (fs.fileFilter (file: file.hasExt "md") ./.)
      (fs.fileFilter (file: file.hasExt "nix") ./.)
    ]
  );

  # Remove the grammar from the languages.toml during the build
  # This is (by default) built in the `grammars.nix` derivation. Anything
  # filtered by `includeGrammarIf` is kept in for the user to build with `hx -g`
  languages_all = readTOML ./languages.toml;
  languages_min = (formats.toml { }).generate "helix-languages.toml" (
    (builtins.removeAttrs languages_all [ "grammar" ])
    // {
      grammar = (builtins.filter (g: !includeGrammarIf g) languages_all.grammar);
    }
  );

  # Next we actually need to build the grammars and the runtime directory
  # that they reside in. It is built by calling the derivation in the
  # grammars.nix file, then taking the runtime directory in the git repo
  # and hooking symlinks up to it.
  grammars = callPackage ./grammars.nix { inherit grammarOverlays includeGrammarIf; };
  runtimeDir = runCommand "helix-runtime" { } ''
    mkdir -p $out
    ln -s ./runtime/* $out
    ln -s ${grammars} $out/grammars
  '';

  # Creates an 'unwrapped' helix without references to the grammars
  helix-unwrapped = rustPlatform.buildRustPackage (self: {
    pname = cargoTOML.package.name;
    version = wsTOML.workspace.package.version;
    src = fs.toSource {
      root = ./.;
      fileset = src;
    };

    cargoLock = {
      lockFile = ./Cargo.lock;
      # This is not allowed in nixpkgs but is very convenient here: it allows us to
      # avoid specifying `outputHashes` here for any git dependencies we might take
      # on temporarily.
      allowBuiltinFetchGit = true;
    };

    patchPhase = ''
      runHook prePatch
      cp -v ${languages_min} languages.toml
      runHook postPatch
    '';

    nativeBuildInputs = [
      installShellFiles
      git
    ];

    buildType = "release";

    # Helix attempts to reach out to the network and get the grammars. Nix doesn't allow this.
    HELIX_DISABLE_AUTO_GRAMMAR_BUILD = "1";
    # So Helix knows what rev it is.
    HELIX_NIX_BUILD_REV = gitRev;

    doCheck = false;
    strictDeps = true;

    # Get all the application stuff in the output directory.
    postInstall = ''
      mkdir -p $out/lib
      installShellCompletion ${./contrib/completion}/hx.{bash,fish,zsh}
      mkdir -p $out/share/{applications,icons/hicolor/{256x256,scalable}/apps}
      cp ${./contrib/Helix.desktop} $out/share/applications/Helix.desktop
      cp ${./logo.svg} $out/share/icons/hicolor/scalable/apps/helix.svg
      cp ${./contrib/helix.png} $out/share/icons/hicolor/256x256/apps/helix.png
    '';

    meta = {
      inherit (wsTOML.workspace.package) homepage;
      inherit (cargoTOML.package) description;
      mainProgram = cargoTOML.package.default-run;
      license = lib.licenses.mpl20;
    };
  });
in
symlinkJoin {
  name = "helix-wrapped-${helix-unwrapped.version}";
  inherit (helix-unwrapped) pname version outputs;

  nativeBuildInputs = [ makeWrapper ];

  paths = [ helix-unwrapped ];

  postBuild = ''
    ${lib.concatMapStringsSep "\n" (
      output: "ln --verbose --symbolic --no-target-directory ${helix-unwrapped.${output}} \$${output}"
    ) (lib.remove "out" helix-unwrapped.outputs)}

    pushd $out/bin
    for f in *; do
      rm $f
      makeWrapper ${helix-unwrapped}/bin/$f $f \
        --set-default HELIX_RUNTIME ${runtimeDir}
    done
    popd
  '';

  passthru = {
    unwrapped = helix-unwrapped;
  };

  meta = helix-unwrapped.meta;
}
