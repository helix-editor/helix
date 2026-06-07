import os
from typing import (
    List,
    Optional,
)


class Example:
    attribute = 1

    def __init__(self, first, second):
        self.first = first
        self.second = second
        self.values = [
            1,
            2,
            3,
        ]
        self.mapping = {
            "a": 1,
            "b": 2,
        }

    def method(self, argument):
        if argument > 0:
            for index in range(argument):
                print(index)
            return argument
        elif argument < 0:
            while argument < 0:
                argument += 1
        else:
            argument = 0

        try:
            value = self.values[argument]
        except IndexError:
            value = None
        except (KeyError, ValueError):
            value = -1
        else:
            value += 1
        finally:
            print("done")

        with open("file") as handle:
            data = handle.read()

        return value


def aligned_call():
    result = some_function(first_argument,
                           second_argument,
                           third_argument)
    return result


def hanging_call():
    result = other_function(
        first_argument,
        second_argument,
    )
    return result


def comprehensions():
    squares = [
        value * value
        for value in range(10)
        if value % 2 == 0
    ]
    return squares


def match_example(command):
    match command:
        case "start":
            return 1
        case "stop":
            return 0
        case _:
            return -1
