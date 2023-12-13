{
  stdenv,
  lib,
  runCommandLocal,
  runCommand,
  yj,
  includeGrammarIf ? _: true,
  grammarOverlays ? [],
  ...
}: let
  # HACK: nix < 2.6 has a bug in the toml parser, so we convert to JSON
  # before parsing
  languages-json = runCommandLocal "languages-toml-to-json" {} ''
    ${yj}/bin/yj -t < ${./languages.toml} > $out
  '';
  languagesConfig =
    if lib.versionAtLeast builtins.nixVersion "2.6.0"
    then builtins.fromTOML (builtins.readFile ./languages.toml)
    else builtins.fromJSON (builtins.readFile (builtins.toPath languages-json));
  isGitGrammar = grammar:
    builtins.hasAttr "source" grammar
    && builtins.hasAttr "git" grammar.source
    && builtins.hasAttr "rev" grammar.source;
  isGitHubGrammar = grammar: lib.hasPrefix "https://github.com" grammar.source.git;
  toGitHubFetcher = url: let
    match = builtins.match "https://github\.com/([^/]*)/([^/]*)/?" url;
  in {
    owner = builtins.elemAt match 0;
    repo = builtins.elemAt match 1;
  };
  gitGrammars = builtins.filter isGitGrammar languagesConfig.grammar;
  buildGrammar = grammar: let
    gh = toGitHubFetcher grammar.source.git;
    sourceGit = builtins.fetchTree {
      type = "git";
      url = grammar.source.git;
      rev = grammar.source.rev;
      ref = grammar.source.ref or "HEAD";
      shallow = true;
    };
    sourceGitHub = builtins.fetchTree {
      type = "github";
      owner = gh.owner;
      repo = gh.repo;
      inherit (grammar.source) rev;
    };
    source =
      if isGitHubGrammar grammar
      then sourceGitHub
      else sourceGit;
  in
    stdenv.mkDerivation {
      # see https://github.com/NixOS/nixpkgs/blob/fbdd1a7c0bc29af5325e0d7dd70e804a972eb465/pkgs/development/tools/parsing/tree-sitter/grammar.nix

      pname = "helix-tree-sitter-${grammar.name}";
      version = grammar.source.rev;

      src = source;
      sourceRoot = if builtins.hasAttr "subpath" grammar.source then
        "source/${grammar.source.subpath}"
      else
        "source";

      dontConfigure = true;

      FLAGS = [
        "-Isrc"
        "-g"
        "-O3"
        "-fPIC"
        "-fno-exceptions"
        "-Wl,-z,relro,-z,now"
      ];

      NAME = grammar.name;

      buildPhase = ''
        runHook preBuild

        if [[ -e src/scanner.cc ]]; then
          $CXX -c src/scanner.cc -o scanner.o $FLAGS
        elif [[ -e src/scanner.c ]]; then
          $CC -c src/scanner.c -o scanner.o $FLAGS
        fi

        $CC -c src/parser.c -o parser.o $FLAGS
        $CXX -shared -o $NAME.so *.o

        ls -al

        runHook postBuild
      '';

      installPhase = ''
        runHook preInstall
        mkdir $out
        mv $NAME.so $out/
        runHook postInstall
      '';

      # Strip failed on darwin: strip: error: symbols referenced by indirect symbol table entries that can't be stripped
      fixupPhase = lib.optionalString stdenv.isLinux ''
        runHook preFixup
        $STRIP $out/$NAME.so
        runHook postFixup
      '';
    };
  grammarsToBuild = builtins.filter includeGrammarIf gitGrammars;
  builtGrammars = builtins.map (grammar: {
    inherit (grammar) name;
    value = buildGrammar grammar;
  }) grammarsToBuild;
  extensibleGrammars =
    lib.makeExtensible (self: builtins.listToAttrs builtGrammars);
  overlayedGrammars = lib.pipe extensibleGrammars
    (builtins.map (overlay: grammar: grammar.extend overlay) grammarOverlays);
  grammarLinks = lib.mapAttrsToList
    (name: artifact: "ln -s ${artifact}/${name}.so $out/${name}.so")
    (lib.filterAttrs (n: v: lib.isDerivation v) overlayedGrammars);
in
  runCommand "consolidated-helix-grammars" {} ''
    mkdir -p $out
    ${builtins.concatStringsSep "\n" grammarLinks}
  ''
