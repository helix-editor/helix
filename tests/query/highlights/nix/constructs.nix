{ pkgs, ... }@args:
#       ^ @variable.parameter.builtin
let
  msg = "${builtins.toString 1}";
#                   ^ @function.builtin
  hasit = args ? foo;
#                ^ @variable.other.member
in
pkgs.mkDerivation {
#    ^ @function
  meta.license = pkgs.lib.mit;
#      ^ @variable.other.member
}
