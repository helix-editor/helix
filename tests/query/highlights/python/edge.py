def h():
    m = obj.method
#           ^ @variable.other.member
    s = f"{obj.compute()}"
#              ^ @function.method
    obj.first().second()
#       ^ @function.method
#                ^ @function.method
