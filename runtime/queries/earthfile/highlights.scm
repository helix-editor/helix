(string_array "," @punctuation.delimiter)
(string_array ["[" "]"] @punctuation.bracket)

(arg_command "ARG" @keyword)
(build_command "BUILD" @keyword)
(cache_command "CACHE" @keyword)
(cmd_command "CMD" @keyword)
(copy_command "COPY" @keyword)
(do_command "DO" @keyword)
(entrypoint_command "ENTRYPOINT" @keyword)
(env_command "ENV" @keyword)
(expose_command "EXPOSE" @keyword)
(from_command "FROM" @keyword)
(from_dockerfile_command "FROM DOCKERFILE" @keyword)
(function_command "FUNCTION" @keyword)
(git_clone_command "GIT CLONE" @keyword)
(host_command "HOST" @keyword)
(import_command "IMPORT" @keyword)
(label_command "LABEL" @keyword)
(let_command "LET" @keyword)
(project_command "PROJECT" @keyword)
(run_command "RUN" @keyword)
(save_artifact_command ["SAVE ARTIFACT" "AS LOCAL"] @keyword)
(save_image_command "SAVE IMAGE" @keyword)
(set_command "SET" @keyword)
(user_command "USER" @keyword)
(version_command "VERSION" @keyword)
(volume_command "VOLUME" @keyword)
(with_docker_command "WITH DOCKER" @keyword)
(workdir_command "WORKDIR" @keyword)

(for_command ["FOR" "IN" "END"] @keyword.control.repeat)
(if_command ["IF" "END"] @keyword.control.conditional)
(elif_block ["ELSE IF"] @keyword.control.conditional)
(else_block ["ELSE"] @keyword.control.conditional)
(import_command ["IMPORT" "AS"] @keyword.control.import)
(try_command ["TRY" "FINALLY" "END"] @keyword.control.exception)
(wait_command ["WAIT" "END"] @keyword.control)


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

(options (_ "=" @operator))
(build_arg "=" @operator)
(arg_command "=" @operator)
(env_command "=" @operator)
(label "=" @operator)
(set_command "=" @operator)
(let_command "=" @operator)
