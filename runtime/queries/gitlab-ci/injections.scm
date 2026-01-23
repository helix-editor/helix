((comment) @injection.content
 (#set! injection.language "comment"))

(block_mapping_pair
  key: (flow_node) @_run (#any-of? @_run "script" "before_script" "after_script" "pre_get_sources_script" "command" "entrypoint")
  value: (flow_node
           (plain_scalar
             (string_scalar) @injection.content)
           (#set! injection.language "bash")))

(block_mapping_pair
  key: (flow_node) @_run (#any-of? @_run "script" "before_script" "after_script" "pre_get_sources_script" "command" "entrypoint")
  value: (block_node
           (block_scalar) @injection.content
           (#set! injection.language "bash")))

(block_mapping_pair
  key: (flow_node) @_run (#any-of? @_run "script" "before_script" "after_script" "pre_get_sources_script" "command" "entrypoint")
  value: (block_node
           (block_sequence
             (block_sequence_item
                (flow_node
                  (plain_scalar
                    (string_scalar) @injection.content))
                (#set! injection.language "bash")))))

(block_mapping_pair
  key: (flow_node) @_run (#any-of? @_run "script" "before_script" "after_script" "pre_get_sources_script" "command" "entrypoint")
  value: (block_node
           (block_sequence
             (block_sequence_item
               (block_node
                  (block_scalar) @injection.content
                  (#set! injection.language "bash"))))))

; e.g.
; ```
; job1:
;   services:
;     entrypoint: ["/usr/local/bin/docker-entrypoint.sh", "-c", 'max_connections=100']
; ```
(block_mapping_pair
  key: (flow_node) @_run (#any-of? @_run "command" "entrypoint")
  value: (flow_node
           (flow_sequence
             (flow_node
               [
                 (double_quote_scalar)
                 (single_quote_scalar)
               ] @injection.content)))
  (#set! injection.language "bash"))


; https://docs.gitlab.com/ci/yaml/#specinputsregex
; ```
; spec:
;   inputs:
;     <input>:
;       regex: <regex>
; ---
; <job>:
;   coverage: <regex>
; ```
(block_mapping_pair
  key: (flow_node) @_key (#any-of? @_key "regex" "coverage")
  value: (flow_node
           [
             (single_quote_scalar) @injection.content
             (double_quote_scalar) @injection.content
             (plain_scalar (string_scalar) @injection.content)
           ])
  (#set! injection.language "regex"))
