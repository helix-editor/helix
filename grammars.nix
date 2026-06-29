{
  stdenv,
  lib,
  linkFarm,
  includeGrammarIf ? _: true,
  grammarOverlays ? [ ],
  callPackage,
}:
let
  # Load the TOML
  languagesConfig = lib.importTOML ./languages.toml;

  # All grammars are git grammar, this is future-proofing
  isGitGrammar = grammar: grammar ? source.git && grammar ? source.rev;

  # grammar builder
  buildGrammar = callPackage ./grammar-build.nix { };

  # If `use-grammars.only` is set, use only those grammars.
  # If `use-grammars.except` is set, use all other grammars.
  # Otherwise use all grammars.
  useGrammar =
    let
      hasOnly = languagesConfig ? use-grammars.only;
      hasExcept = languagesConfig ? use-grammars.except;
    in
    grammar:
    if hasOnly then
      builtins.elem grammar.name languagesConfig.use-grammars.only
    else if hasExcept then
      !(builtins.elem grammar.name languagesConfig.use-grammars.except)
    else
      true;

  # Filter the grammars to the ones that we must build
  grammarsToBuild = builtins.filter (
    g: (useGrammar g) && (isGitGrammar g) && (includeGrammarIf g)
  ) languagesConfig.grammar;

  # Build an attrset of grammars that looks like:
  # {
  #    ${name} = built-grammar;
  # }
  builtGrammars = lib.foldr (
    grammar: attrset:
    attrset
    // {
      ${grammar.name} = buildGrammar {
        inherit (grammar) name source;
      };
    }
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
