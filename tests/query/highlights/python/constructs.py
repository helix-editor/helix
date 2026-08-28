import os
#      ^ @namespace
class Widget:
#     ^ @type
    def method(self, count: int) -> bool:
#       ^ @function
#              ^ @variable.builtin
#                    ^ @variable.parameter
#                           ^ @type.builtin
        return self.helper(count)
#                   ^ @function.method
