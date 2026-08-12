((comment) @injection.content
 (#set! injection.language "comment"))

; https://docs.tilt.dev/api.html#api.local
; https://docs.tilt.dev/api.html#api.run
; e.g. `local("git rev-parse --show-toplevel")`
(call
  function: (identifier) @_function (#any-of? @_function "local" "run")
  arguments: (argument_list . (string (string_content) @injection.content))
  (#set! injection.language "bash"))

; https://docs.tilt.dev/api.html#api.local_resource
; https://docs.tilt.dev/api.html#api.custom_build
; e.g.
; ```
; custom_build(
;   'gcr.io/my-project/frontend-server',
;   'docker build -t $EXPECTED_REF .',
;   ['.'],
; )
; ```
(call
  function: (identifier) @_function (#any-of? @_function "custom_build" "local_resource")
  arguments: (argument_list . (string) (string (string_content) @injection.content))
  (#set! injection.language "bash"))

; https://docs.tilt.dev/api.html#api.k8s_custom_deploy
; e.g.
; ```
; local_resource(
;   'local-myserver',
;   cmd='go build ./cmd/myserver',
;   serve_cmd='./myserver --port=8001',
;   deps=['cmd/myserver']
; )
; ```
(call
  arguments: (argument_list
    (keyword_argument
      name: (identifier) @_keyword_arg (#any-of? @_keyword_arg
        "cmd"
        "serve_cmd"
        "apply_cmd"
        "delete_cmd"
        "cmd_bat"
        "apply_cmd_bat"
        "serve_cmd_bat"
        "delete_cmd_bat"
        "command"
        "command_bat"
        "entrypoint")
      value: (string (string_content) @injection.content)))
  (#set! injection.language "bash"))
