; Sema injection queries for Helix
; Enables comment-aware features (e.g. TODO highlighting in comments).

([(comment) (block_comment)] @injection.content
  (#set! injection.language "comment"))
