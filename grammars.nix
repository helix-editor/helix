{
  stdenv,
  lib,
  runCommand,
  tree-sitter,
  includeGrammarIf ? _: true,
  grammarOverlays ? [ ],
  ...
}:
let
  languagesConfig = builtins.fromTOML (builtins.readFile ./languages.toml);
  isGitGrammar =
    grammar:
    builtins.hasAttr "source" grammar
    && builtins.hasAttr "git" grammar.source
    && builtins.hasAttr "rev" grammar.source;
  isGitHubGrammar = grammar: lib.hasPrefix "https://github.com" grammar.source.git;
  toGitHubFetcher =
    url:
    let
      match = builtins.match "https://github\\.com/([^/]*)/([^/]*)/?" url;
    in
    {
      owner = builtins.elemAt match 0;
      repo = builtins.elemAt match 1;
    };
  # If `use-grammars.only` is set, use only those grammars.
  # If `use-grammars.except` is set, use all other grammars.
  # Otherwise use all grammars.
  useGrammar =
    grammar:
    if languagesConfig ? use-grammars.only then
      builtins.elem grammar.name languagesConfig.use-grammars.only
    else if languagesConfig ? use-grammars.except then
      !(builtins.elem grammar.name languagesConfig.use-grammars.except)
    else
      true;
  grammarsToUse = builtins.filter useGrammar languagesConfig.grammar;
  gitGrammars = builtins.filter isGitGrammar grammarsToUse;
  buildGrammar =
    grammar:
    let
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
      source = if isGitHubGrammar grammar then sourceGitHub else sourceGit;
      # `buildGrammar` errors if the version doesn't match `tree-sitter.json`
      treeSitterJSON = "${source}/${grammar.source.subpath or "."}/tree-sitter.json";
      srcVersion =
        if builtins.pathExists treeSitterJSON then
          (lib.importJSON treeSitterJSON).metadata.version or "0"
        else
          "0";
    in
    # see https://github.com/NixOS/nixpkgs/blob/79a616fd724cfc5c3ea28d5ca6d1355d196b07e9/pkgs/by-name/tr/tree-sitter/grammars/build-grammar.nix
    tree-sitter.passthru.buildGrammar {
      language = grammar.name;
      version = "${srcVersion}+${source.shortRev}";
      src = source;
      location = grammar.source.subpath or null;
    };
  grammarsToBuild = builtins.filter includeGrammarIf gitGrammars;
  builtGrammars = builtins.map (grammar: {
    inherit (grammar) name;
    value = buildGrammar grammar;
  }) grammarsToBuild;
  extensibleGrammars = lib.makeExtensible (self: builtins.listToAttrs builtGrammars);
  overlaidGrammars = lib.pipe extensibleGrammars (
    builtins.map (overlay: grammar: grammar.extend overlay) grammarOverlays
  );
  linkedGrammars = lib.filterAttrs (n: v: lib.isDerivation v) overlaidGrammars;
  sharedLibExtension = stdenv.hostPlatform.extensions.sharedLibrary;
  grammarLinks = lib.mapAttrsToList (
    name: artifact:
    "ln -s ${artifact}/${artifact.passthru.SHARED_LIB or "parser"} $out/${name}${sharedLibExtension}"
  ) linkedGrammars;
in
runCommand "consolidated-helix-grammars"
  {
    passthru.names = builtins.attrNames linkedGrammars;
  }
  ''
    mkdir -p $out
    ${builtins.concatStringsSep "\n" grammarLinks}
  ''
