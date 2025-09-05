((comment) @injection.content
 (#set! injection.language "comment"))

; The remaining code in this file incorporates work covered by the following
; copyright and permission notice:
;
;   Copyright 2023 the nvim-treesitter authors
;
;   Licensed under the Apache License, Version 2.0 (the "License");
;   you may not use this file except in compliance with the License.
;   You may obtain a copy of the License at
;
;       http://www.apache.org/licenses/LICENSE-2.0
;
;   Unless required by applicable law or agreed to in writing, software
;   distributed under the License is distributed on an "AS IS" BASIS,
;   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
;   See the License for the specific language governing permissions and
;   limitations under the License.

; Modified for Helix from https://github.com/nvim-treesitter/nvim-treesitter/blob/master/queries/yaml/injections.scm

;; GitHub actions: run
;; Buildkite: command, commands
(block_mapping_pair
  key: (flow_node) @_run (#any-of? @_run "run" "command" "commands")
  value: (flow_node
           (plain_scalar
             (string_scalar) @injection.content)
           (#set! injection.language "bash")))

(block_mapping_pair
  key: (flow_node) @_run (#any-of? @_run "run" "command" "commands")
  value: (block_node
           (block_scalar) @injection.content
           (#set! injection.language "bash")))

(block_mapping_pair
  key: (flow_node) @_run (#any-of? @_run "run" "command" "commands")
  value: (block_node
           (block_sequence
             (block_sequence_item
                (flow_node
                  (plain_scalar
                    (string_scalar) @injection.content))
                (#set! injection.language "bash")))))

(block_mapping_pair
  key: (flow_node) @_run (#any-of? @_run "run" "command" "commands")
  value: (block_node
           (block_sequence
             (block_sequence_item
               (block_node
                  (block_scalar) @injection.content
                  (#set! injection.language "bash"))))))
