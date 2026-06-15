{
  lib,
  stdenv,
}:
let
  getForgeAttrs =
    forge: url:
    let
      match = builtins.match "https://${lib.escapeRegex forge}/([^/]*)/([^/]*)/?" url;
    in
    {
      owner = builtins.elemAt match 0;
      repo = builtins.elemAt match 1;
    };

in
# See https://github.com/NixOS/nixpkgs/blob/c9e6cd244e50f67d2644de7b254fb9208af8724e/pkgs/by-name/tr/tree-sitter/grammars/build-grammar.nix
lib.extendMkDerivation {
  constructDrv = stdenv.mkDerivation;

  excludeDrvArgNames = [
    "name"
    "source"
  ];

  extendDrvArgs =
    finalAttrs:
    {
      name,
      source,
    }:
    let
      ghAttrs = getForgeAttrs "github.com" source.git;
      glAttrs = getForgeAttrs "gitlab.com" source.git;
      cbAttrs = getForgeAttrs "codeberg.org" source.git;
      shAttrs = getForgeAttrs "git.sr.ht" source.git;
    in
    {
      pname = "helix-tree-sitter-${name}";
      version = source.rev;

      # Avoid using fetchgit, and do not use fetchtree.
      # fetchtree is only avaliable if flakes are enabled.
      # Both are a bit jank (fetchgit is implemented via fetchtree these days) and will spam the terminal every time helix is built.
      # And I recommend avoiding fetchTarball as that unpacks the tarball in the store, which is rarely nesscesary and increases
      # storage size at rest.
      src =
        if lib.hasPrefix "https://github.com" source.git then
          builtins.fetchurl {
            # Template:
            #   https://github.com/helix-editor/helix/archive/43bf7c2dc219606c64003aef21151f49f48d0939.tar.gz
            url = "https://github.com/${ghAttrs.owner}/${ghAttrs.repo}/archive/${finalAttrs.version}.tar.gz";
          }
        else if lib.hasPrefix "https://codeberg.org" source.git then
          builtins.fetchurl {
            # Template
            # https://codeberg.org/microcad/microcad/archive/31d05a1afd9850b6c7353d31f7d1e45f823c6d43.tar.gz
            url = "https://codeberg.org/${cbAttrs.owner}/${cbAttrs.repo}/archive/${finalAttrs.version}.tar.gz";
          }
        else if lib.hasPrefix "https://gitlab.com" source.git then
          builtins.fetchurl {
            # Template
            # https://gitlab.com/famedly/conduit/-/archive/a2dbc6fe6de27e3241c71cf63c2b6d35efe8da67/conduit-a2dbc6fe6de27e3241c71cf63c2b6d35efe8da67.tar.gz
            url = "https://gitlab.com/${glAttrs.owner}/${glAttrs.repo}/-/archive/${finalAttrs.version}/${name}-${finalAttrs.version}.tar.gz";
          }
        else if lib.hasPrefix "https://git.sr.ht" source.git then
          builtins.fetchurl {
            # Template
            # https://git.sr.ht/~kennylevinsen/greetd/archive/d3b45e7398d3eed65b39c532c713ea052a5b9278.tar.gz
            url = "https://git.sr.ht/${shAttrs.owner}/${shAttrs.repo}/archive/${finalAttrs.version}.tar.gz";
          }
        else
          fetchGit {
            url = source.git;
            rev = finalAttrs.version;
            ref = source.ref or "HEAD";
            shallow = true;
          };

      # sourceRoot is annoying with builtin fetchers
      # And postUnpack is cursed
      preBuild = lib.optionalString (source ? subpath) ''
        cd ${source.subpath}
      '';

      dontConfigure = true;

      __structuredAttrs = true;
      strictDeps = true;

      # Strip failed on darwin: strip: error: symbols referenced by indirect symbol table entries that can't be stripped
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
