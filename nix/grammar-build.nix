{
  lib,
  stdenv,
}:

# Based on https://github.com/NixOS/nixpkgs/blob/c9e6cd244e50f67d2644de7b254fb9208af8724e/pkgs/by-name/tr/tree-sitter/grammars/build-grammar.nix
lib.extendMkDerivation {
  constructDrv = stdenv.mkDerivation;

  excludeDrvArgNames = [
    "name"
    "source"
    "subpath"
  ];

  extendDrvArgs =
    finalAttrs:
    {
      name,
      version,
      subpath ? null,
      src,
    }:
    {
      pname = "helix-tree-sitter-${name}";

      preBuild = lib.optionalString (!(isNull subpath)) ''
        cd ${lib.escapeShellArg subpath}
      '';

      dontConfigure = true;

      __structuredAttrs = true;
      strictDeps = true;

      # Strip failed on Darwin: strip: error: symbols referenced by indirect symbol table entries that can't be stripped.
      stripDebugList = [ "parser" ];

      FLAGS = [
        "-Isrc"
        "-g"
        "-O3"
        "-fPIC"
        "-fno-exceptions"
        "-Wl,-z,relro,-z,now"
      ];

      SHARED_LIB = name + stdenv.hostPlatform.extensions.sharedLibrary;

      buildPhase = ''
        runHook preBuild

        if [[ -e src/scanner.cc ]]; then
          $CXX -c src/scanner.cc -o scanner.o ''${FLAGS[@]}
        elif [[ -e src/scanner.c ]]; then
          $CC -c src/scanner.c -o scanner.o ''${FLAGS[@]}
        fi

        $CC -c src/parser.c -o parser.o ''${FLAGS[@]}
        rm -rf parser

        $CXX -shared -o $SHARED_LIB *.o

        runHook postBuild
      '';

      installPhase = ''
        runHook preInstall
        mkdir $out
        mv $SHARED_LIB $out/
        runHook postInstall
      '';
    };
}
