# Flake's devShell for non-flake-enabled nix instances
let
  compat = builtins.fetchGit {
    url = "https://github.com/edolstra/flake-compat.git";
    rev = "b4a34015c698c7793d592d66adbab377907a2be8";
  };
in
  (import compat {src = ./.;}).shellNix.default
