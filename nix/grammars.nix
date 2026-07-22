{
  stdenv,
  lib,
  linkFarm,
  fetchurl,
  includeGrammarIf ? _: true,
  grammarOverlays ? [ ],
  callPackage,
}:
let
  # Load the TOML
  helixGrammarConfig = lib.importTOML ../languages.toml;

  # Load the grammar FOD hashes.
  nixGrammarLock = lib.importJSON ./grammar-sources.json;

  # All grammars are git grammars currently, this is future-proofing
  isGitGrammar = grammar: grammar ? source.git && grammar ? source.rev;

  # grammar builder
  buildGrammar = callPackage ./grammar-build.nix { };

  # If `use-grammars.only` is set, use only those grammars.
  # If `use-grammars.except` is set, use all other grammars.
  # Otherwise use all grammars.
  useGrammar =
    let
      hasOnly = helixGrammarConfig ? use-grammars.only;
      hasExcept = helixGrammarConfig ? use-grammars.except;
    in
    grammar:
    if hasOnly then
      builtins.elem grammar.name helixGrammarConfig.use-grammars.only
    else if hasExcept then
      !(builtins.elem grammar.name helixGrammarConfig.use-grammars.except)
    else
      true;

  # Filter the grammars to the ones that we must build
  grammarsToBuild = builtins.filter (
    g: (useGrammar g) && (isGitGrammar g) && (includeGrammarIf g)
  ) helixGrammarConfig.grammar;

  # Build an attrset of grammars that looks like:
  # {
  #    ${name} = built-grammar;
  # }
  #
  # Do not error if a grammar is missing or the files are mismatched.
  # Helix will work fine without all grammars.
  builtGrammars = lib.foldr (
    grammar: grammarPkgSet:
    let
      name = grammar.name;
    in
    if nixGrammarLock ? ${name} then
      let
        metadata = nixGrammarLock.${name};
        rev =
          lib.warnIf (grammar.source.rev != metadata.rev)
            "Nix grammar lock file and Helix languages.toml git hashes do not match for ${name}. Will use Nix-locked grammar."
            metadata.rev;
      in
      grammarPkgSet
      // {
        ${name} = buildGrammar {
          inherit name;
          version = rev;
          src = fetchurl {
            inherit (metadata) url hash;
          };
          subpath = grammar.source.subpath or null;
        };
      }
    else
      builtins.warn "Nix grammar lock file does not contain ${name}, skipping." grammarPkgSet
  ) { } grammarsToBuild;

  # Combine all overlays to one
  composedOverlays = lib.composeManyExtensions grammarOverlays;

  # Apply the overlays
  extensibleGrammars = lib.pipe builtGrammars [
    # We need to pretend this is a function
    lib.const
    # Extend the grammars with the overlays
    (lib.extends composedOverlays)
    # Make the overall pkgset extensible
    lib.makeExtensible
  ];

  extension = stdenv.hostPlatform.extensions.sharedLibrary;
in
# The attrset has some functions since it was extensible,
# so first filter those out.
# Then we must append the shared library extension to all the
# name so the symlinks have the correct name
linkFarm "consolidated-helix-grammars" (
  lib.mapAttrs' (name: lib.nameValuePair (name + extension)) (
    lib.filterAttrs (lib.const lib.isDerivation) extensibleGrammars
  )
)
