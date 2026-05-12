(string_array "," @punctuation.delimiter)
(string_array ["[" "]"] @punctuation.bracket)

[
    "ARG"
    "AS LOCAL"
    "BUILD"
    "CACHE"
    "CMD"
    "COPY"
    "DO"
    "ENTRYPOINT"
    "ENV"
    "EXPOSE"
    "FROM DOCKERFILE"
    "FROM"
    "FUNCTION"
    "GIT CLONE"
    "HOST"
    "IMPORT"
    "LABEL"
    "LET"
    "PROJECT"
    "RUN"
    "SAVE ARTIFACT"
    "SAVE IMAGE"
    "SET"
    "USER"
    "VERSION"
    "VOLUME"
    "WORKDIR"
] @keyword

(for_command ["FOR" "IN" "END"] @keyword.control.repeat)

(if_command ["IF" "END"] @keyword.control.conditional)
(elif_block ["ELSE IF"] @keyword.control.conditional)
(else_block ["ELSE"] @keyword.control.conditional)

(import_command ["IMPORT" "AS"] @keyword.control.import)

(try_command ["TRY" "FINALLY" "END"] @keyword.control.exception)

(wait_command ["WAIT" "END"] @keyword.control)
(with_docker_command ["WITH DOCKER" "END"] @keyword.control)

[
    (comment)
    (line_continuation_comment)
] @comment

(line_continuation) @operator

[
    (target_ref)
    (target_artifact)
    (function_ref)
] @function

(target (identifier) @function)

[
    (double_quoted_string)
    (single_quoted_string)
] @string
(unquoted_string) @string.special
(escape_sequence) @constant.character.escape

(variable) @variable
(expansion ["$" "{" "}" "(" ")"] @punctuation.special)
(build_arg) @variable
(options (_) @variable.parameter)

"=" @operator
