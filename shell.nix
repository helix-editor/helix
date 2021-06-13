# Flake's devShell for non-flake-enabled nix instances
let
  src = (builtins.fromJSON (builtins.readFile ./flake.lock)).nodes.flakeCompat.locked;
  compat = fetchTarball { url = "https://github.com/edolstra/flake-compat/archive/${src.rev}.tar.gz"; sha256 = src.narHash; };
in
(import compat { src = ./.; }).shellNix.default
