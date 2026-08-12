{
  x = map double items;
#     ^ @function.builtin
#         ^ @function
#                ^ @variable
  y = builtins.length items;
#     ^ @constant.builtin
#              ^ @function.builtin
  z = value |> lib.foo;
#                  ^ @function
  w = lib.foo <| value;
#         ^ @function
  merged = lib.foldr lib.recursiveUpdate { } foo;
#              ^ @function
#                        ^ @function
#                                              ^ @variable
  generated = lib.forEach items lib.makeThing;
#                   ^ @function
#                           ^ @variable
#                                     ^ @function
  walked = lib.mapAttrsRecursiveCond lib.recurse lib.transform attrs;
#                ^ @function
#                                        ^ @function
#                                                    ^ @function
#                                                                ^ @variable
  grouped = lib.groupBy' lib.merge { } lib.keyFor items;
#                ^ @function
#                             ^ @function
#                                           ^ @function
#                                                  ^ @variable
  traced = lib.traceFnSeqN 2 "value" lib.render value;
#               ^ @function
#                                         ^ @function
#                                                ^ @variable
  local = let transform = x: x; in map transform items;
#                                  ^ @function.builtin
#                                      ^ @function
  localSecond = let transform = x: x; in lib.forEach items transform;
#                                              ^ @function
#                                                    ^ @variable
#                                                          ^ @function
  extension = lib.toExtension overrides;
#                 ^ @function
#                             ^ @variable
  package = lib.callPackageWith pkgs packageFile { };
#                ^ @function
#                                ^ @variable
}
