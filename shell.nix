# Flake's devShell for non-flake-enabled nix instances
let
  compat = builtins.fetchTarball {
    url = "https://github.com/edolstra/flake-compat/archive/b4a34015c698c7793d592d66adbab377907a2be8.tar.gz";
    sha256 = "sha256:1qc703yg0babixi6wshn5wm2kgl5y1drcswgszh4xxzbrwkk9sv7";
  };
in
  (import compat {src = ./.;}).shellNix.default
