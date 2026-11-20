{
  stdenv,
  pkgs,
  lib,
  runCommand,
  tree-sitter,
  # Note: this uses a different input now, but `name` is still the same.
  includeGrammarIf ? _: true,
  grammarOverlays ? [ ],
  grammarsOverlay ? (_: _: { }),
  ...
}:
let
  nixGrammarLock = builtins.fromJSON (builtins.readFile ./grammar-sources.json);
  buildGrammar =
    name: grammar:
    let
      src = pkgs.${grammar.nurl.fetcher} grammar.nurl.args;
      # `buildGrammar` errors if the version doesn't match `tree-sitter.json`
      treeSitterJSON = "${src}/${grammar.subpath or "."}/tree-sitter.json";
      srcVersion =
        if builtins.pathExists treeSitterJSON then
          (lib.importJSON treeSitterJSON).metadata.version or "0"
        else
          "0";
    in
    # see https://github.com/NixOS/nixpkgs/blob/79a616fd724cfc5c3ea28d5ca6d1355d196b07e9/pkgs/by-name/tr/tree-sitter/grammars/build-grammar.nix
    tree-sitter.passthru.buildGrammar {
      inherit src;
      language = name;
      version = "${srcVersion}+${lib.sources.shortRev grammar.nurl.args.rev}";
      location = grammar.subpath or null;
    };
  grammarsToBuild = builtins.filterAttrs (
    name: grammar: includeGrammarIf (grammar // { inherit name; })
  ) nixGrammarLock;
  builtGrammars = lib.mapAttrs' (name: grammar: {
    inherit name;
    value = buildGrammar name grammar;
  }) grammarsToBuild;
  extensibleGrammars = lib.makeExtensible (self: builtGrammars);
  overlaidGrammars = lib.pipe extensibleGrammars (
    builtins.map (overlay: grammar: grammar.extend overlay) ([ grammarsOverlay ] ++ grammarOverlays)
  );
  linkedGrammars = lib.filterAttrs (n: v: lib.isDerivation v) overlaidGrammars;
  sharedLibExtension = stdenv.hostPlatform.extensions.sharedLibrary;
  grammarLinks = lib.mapAttrsToList (
    name: artifact:
    "ln -s ${artifact}/${artifact.passthru.SHARED_LIB or "parser"} $out/${name}${sharedLibExtension}"
  ) linkedGrammars;
in
runCommand "consolidated-helix-grammars" { passthru.names = builtins.attrNames linkedGrammars; } ''
  mkdir -p $out
  ${builtins.concatStringsSep "\n" grammarLinks}
''
