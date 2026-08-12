def f(x) do
  helper(x)
# ^ @function
  x |> Mod.doThing()
#          ^ @function
  "#{String.upcase(x)}"
#            ^ @function
  is_binary(x)
# ^ @function
end
