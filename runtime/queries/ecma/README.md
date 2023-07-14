# Inheritance model for ecma-based languages

Ecma-based languages share many traits. Because of this we want to share as many queries as possible while avoiding nested inheritance that can make query behaviour unpredictable due to unexpected precedence.

To achieve that, there are "public" and "private" versions for javascript, jsx, and typescript query files, that share the same name, but the "private" version name starts with an underscore (with the exception of ecma, that already exists as a sort of base "private" language). This allows the "private" versions to host the specific queries of the language excluding any `; inherits` statement, in order to make them safe to be inherited by the "public" version of the same language and other languages as well. The tsx language doesn't have a "private" version given that currently it doesn't need to be inherited by other languages.

| Language   | Inherits from           |
| ---------- | ----------------------- |
| javascript | _javascript, ecma       |
| jsx        | _jsx, _javascript, ecma |
| typescript | _typescript, ecma       |
| tsx        | _jsx, _typescript, ecma |

If you intend to add queries to any of the ecma-based languages above, make sure you add them to the correct private language they belong to, so that other languages down the line can benefit from them.
