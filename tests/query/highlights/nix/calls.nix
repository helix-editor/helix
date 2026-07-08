{
  x = map double items;
#     ^ @function.builtin
  y = builtins.length items;
#     ^ @constant.builtin
#              ^ @function.builtin
  z = value |> lib.foo;
#                  ^ @function
  w = lib.foo <| value;
#         ^ @function
}
