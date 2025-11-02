# top comment
# top comment
# top comment


class Fizz:
    def __init__(self, a):
        """
        doc comment
        doc comment
        doc comment
        """
        b = a + a
        self.b = b

    def f(self):
        a = self.b // 2
        c = a + b


# comment
# comment
# comment
def f(a, b):
    """
    doc comment
    doc comment
    doc comment
    """

    class Nested:
        def __init__(self, b):
            self.b = b
            # really nested comment
            # really nested comment
            # really nested comment
            print("log")

        def f(self):
            """
            really nested doc comment
            really nested doc comment
            really nested doc comment
            """

            class ReallyNested:
                def f(self):
                    print(1 + 1)
                    print(2 + 2)

                def g(self):
                    print(1 + 1)
                    print(2 + 2)

            print("log")
            print(f"b = {self.b}")  # interfering comment

    def nested(a, b):
        # interfering comment
        # interfering comment
        # interfering comment
        """
        nested doc comment
        nested doc comment
        nested doc comment
        """
        print("log")
        print(a + b)

    c = a + b
    d = a + b + c
    # nested comment
    # nested comment
    # nested comment
    return c + d  # interfering comment