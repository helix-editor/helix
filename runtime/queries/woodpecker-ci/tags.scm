; select steps
; e.g.
; ```
; steps:
;   - name: build-image
;     image: docker
;     commands:
;       - docker build --rm -t local/project-image .
;     volumes:
;       - /var/run/docker.sock:/var/run/docker.sock
; ```
(block_mapping_pair
  key: (_) @_key (#eq? @_key "steps")
  value: (block_node
           (block_sequence
             (block_sequence_item
               (block_node) @definition.struct))))
