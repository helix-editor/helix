function f(x)
    obj.doThing(x)
#       ^ @function
    helper(x)
#   ^ @function
    y = obj.field
#           ^ @variable.other.member
end
