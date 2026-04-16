(require-builtin helix/core/treesitter as helix.ts.)

(provide TSTree?)
;;@doc
;;Check if the given value is a treesitter tree
(define TSTree? helix.ts.TSTree?)

(provide TSNode?)
;;@doc
;;Check if the given value is a treesitter node
(define TSNode? helix.ts.TSNode?)

(provide TSQueryLoader?)
;;@doc
;;Check if the given value is a treesitter query loader
(define TSQueryLoader? helix.ts.TSQueryLoader?)

(provide TSSyntax?)
;;@doc
;;Check if the given value is a treesitter query loader
(define TSSyntax? helix.ts.TSSyntax?)

(provide TSQuery?)
;;@doc
;;Check if the given value is a treesitter query
(define TSQuery? helix.ts.TSQuery?)

(provide TSMatch?)
;;@doc
;;Check if the given value is a treesitter match
(define TSMatch? helix.ts.TSMatch?)

(provide tsquery-loader)
;;@doc
;; Create a query loader with the given function
;;
;; ```scheme
;; (tsquery-loader fun) -> TSQueryLoader?
;; ```
;;
;; * fun : (-> string?) -> (or TSQuery? bool?)
(define (tsquery-loader fun)
  (if (= (function-arity fun) 1)
      (helix.ts.tsquery-loader (#%closure->boxed-function fun))
      (error "provided function *must* have arity 1 and accept a `string?`")))

(provide tstree->root)
;;@doc
;; Get the root node of the TreeSitter Tree
;;
;; ```scheme
;; (tstree->root tree) -> TSNode?
;; ```
;;
;; * tree : TSTree?
(define tstree->root helix.ts.tstree->root)

(provide tsnode->tstree)
;;@doc
;; Get the root tree object from the given node
;; ```scheme
;; (tsnode->tstree node) -> TSTree?
;; ```
;;
;; * node : TSNode?
(define tsnode->tstree helix.ts.tsnode->tstree)

(provide tsnode-parent)
;;@doc
;; Get the root node of the TreeSitter Tree, returns #f if there is no parent
;; ```scheme
;; (tsnode-parent node) -> (or TSNode? bool?)
;; ```
;;
;; * node : TSNode?
(define tsnode-parent helix.ts.tsnode-parent)

(provide tsnode-children)
;;@doc
;; Get the given node's children
;; ```scheme
;; (tsnode-children node) -> (listof TSNode?)
;; ```
;;
;; * node : TSNode?
(define tsnode-children helix.ts.tsnode-children)

(provide tsnode-named-children)
;;@doc
;; Get the given node's (named) children
;; ```scheme
;; (tsnode-named-children node) -> (listof TSNode?)
;; ```
;;
;; * node : TSNode?
(define tsnode-named-children helix.ts.tsnode-named-children)

(provide tsnode-within-byte-range?)
;;@doc
;; Return whether or not the given node is within the byte range
;; ```scheme
;; (tsnode-within-byte-range node lower upper) -> bool?
;; ```
;;
;; * node : TSNode?
;; * lower : (and positive? int?)
;; * upper : (and positive? int?)
(define tsnode-within-byte-range? helix.ts.tsnode-within-byte-range?)

(provide tsnode-descendant-byte-range)
;;@doc
;; Return a descendant node with the largest byte range within the given range on the tree (#f if one doesn't exist)
;; ```scheme
;; (tsnode-descendant-byte-range node lower upper) -> (or TSNode? bool?)
;; ```
;;
;; * node : TSNode?
;; * lower : (and positive? int?)
;; * upper : (and positive? int?)
(define tsnode-descendant-byte-range helix.ts.tsnode-descendant-byte-range)

(provide tsnode-named-descendant-byte-range)
;;@doc
;; Return a (named) descendant node with the largest byte range within the given range on the tree (#f if one doesn't exist)
;; ```scheme
;; (tsnode-named-descendant-byte-range node lower upper) -> (or TSNode? bool?)
;; ```
;;
;; * node : TSNode?
;; * lower : (and positive? int?)
;; * upper : (and positive? int?)
(define tsnode-named-descendant-byte-range helix.ts.tsnode-named-descendant-byte-range)

(provide tsnode-kind)
;;@doc
;; Get the `kind` of a given node as a string
;; ```scheme
;; (tsnode-kind node) -> string?
;; ```
;;
;; * node : TSNode?
(define tsnode-kind helix.ts.tsnode-kind)

(provide tsnode-named?)
;;@doc
;; Returns whether or not the given node is named
;; ```scheme
;; (tsnode-named? node) -> bool?
;; ```
;;
;; * node : TSNode?
(define tsnode-named? helix.ts.tsnode-named?)

(provide tsnode-extra?)
;;@doc
;; Returns whether or not the given node is extra
;; ```scheme
;; (tsnode-extra? node) -> bool?
;; ```
;;
;; * node : TSNode?
(define tsnode-extra? helix.ts.tsnode-extra?)

(provide tsnode-missing?)
;;@doc
;; Returns whether or not the given node is missing
;; ```scheme
;; (tsnode-missing? node) -> bool?
;; ```
;;
;; * node : TSNode?
(define tsnode-missing? helix.ts.tsnode-missing?)

(provide tsnode-visible?)
;;@doc
;; Returns whether or not the given node is visible
;; ```scheme
;; (tsnode-visible? node) -> bool?
;; ```
;;
;; * node : TSNode?
(define tsnode-visible? helix.ts.tsnode-visible?)

(provide tsnode-print-tree)
;;@doc
;; Pretty print the given TSNode's subtree
;; ```scheme
;; (tsnode-print-tree node) -> string?
;; ```
;;
;; * node : TSNode?
(define tsnode-print-tree helix.ts.tsnode-print-tree)

(provide tsnode-end-byte)
;;@doc
;; Get the end byte idx of the TSNode's range
;; ```scheme
;; (tsnode-end-byte node) -> (and positive? int?)
;; ```
;;
;; * node : TSNode?
(define tsnode-end-byte helix.ts.tsnode-end-byte)

(provide tsnode-start-byte)
;;@doc
;; Get the start byte idx of the TSNode's range
;; ```scheme
;; (tsnode-start-byte node) -> (and positive? int?)
;; ```
;;
;; * node : TSNode?
(define tsnode-start-byte helix.ts.tsnode-start-byte)

(provide tsmatch-captures)
;;@doc
;; Get a list of captures
;; ```scheme
;; (tsmatch-captures match) -> (listof string?)
;; ```
;;
;; * match : TSMatch?
(define tsmatch-captures helix.ts.tsmatch-captures)

(provide tsmatch-capture)
;;@doc
;; Get a list of captures from the given capture group
;; ```scheme
;; (tsmatch-capture match capture) -> (or (listof TSNode?) bool?)
;; ```
;;
;; * match : TSMatch?
;; * capture : string?
(define tsmatch-capture helix.ts.tsmatch-capture)

(provide tssyntax->tree-byte-range)
;;@doc
;; Get the subtree from the given byte range and TSSyntax (#f if no tree is found/available)
;; ```scheme
;; (tssyntax->tree-byte-range syntax lower upper) -> (or TSTree? bool?)
;; ```
;;
;; * syntax : TSSyntax?
;; * lower : (and positive? int?)
;; * upper : (and positive? int?)
(define tssyntax->tree-byte-range helix.ts.tssyntax->tree-byte-range)

(provide tssyntax->layers-byte-range)
;;@doc
;; Get the corresponding parse trees/layers that contain the given byte range
;; ```scheme
;; (tssyntax->layers-byte-range syntax lower upper) -> (or (listof TSTree?) bool?)
;; ```
;; * syntax : TSSyntax?
;; * lower : (and int? positive?)
;; * upper : (and int? positive?)
(define tssyntax->layers-byte-range helix.ts.tssyntax->layers-byte-range)

(provide tssyntax->tree)
;;@doc
;; Get the root subtree from the given TSSyntax
;; ```scheme
;; (tssyntax->tree syntax) -> TSTree?
;; ```
;;
;; * syntax : TSSyntax?
(define tssyntax->tree helix.ts.tssyntax->tree)

;; ctx functions

(provide document->tree)
;;@doc
;;
(define document->tree helix.ts.document->tree)

(provide document->tree-byte-range)
;;@doc
;; Get the full treesitter tree with a byte range from the given document (not necessarily the full parse tree)
;; ```scheme
;; (document->tree-byte-range doc-id lower upper) -> (or TSTree? bool?)
;; ```
;; * doc-id : DocumentId?
;; * lower : (and int? positive?)
;; * upper : (and int? positive?)
(define document->tree-byte-range helix.ts.document->tree-byte-range)

(provide document->layers-byte-range)
;;@doc
;; Get the corresponding parse trees/layers that contain the given byte range
;; ```scheme
;; (document->layers-byte-range doc-id lower upper) -> (listof TSTree?)
;; ```
;; * doc-id : DocumentId?
;; * lower : (and int? positive?)
;; * upper : (and int? positive?)
(define document->layers-byte-range helix.ts.document->layers-byte-range)

(provide tstree->language)
;;@doc
;; Get the language as a string from a given TSTree
;; ```scheme
;; (tstree->language tree) -> string?
;; ```
;; * tree : TSTree?
(define tstree->language helix.ts.tstree->language)

(provide query-document)
;;@doc
;; Run a treesitter query on a given document's parse tree
;; ```scheme
;; (query-document query-loader doc-id) -> (or TSMatch? bool?)
;; ```
;; * query-loader : TSQueryLoader?
;; * doc-id : DocumentId?
(define query-document helix.ts.query-document)

(provide query-document-byte-range)
;;@doc
;; Run a treesitter query on a given document's parse tree with a range (byte indices)
;; ```scheme
;; (query-document-byte-range query-loader doc-id lower upper) -> (or TSMatch? bool?)
;; ```
;; * query-loader : TSQueryLoader?
;; * doc-id : DocumentId?
;; * lower : (and int? positive?)
;; * upper : (and int? positive?)
(define query-document-byte-range helix.ts.query-document-byte-range)

(provide string->tsquery)
;;@doc
;; Create a new treesitter query given a language name and source
;; ```scheme
;; (string->tsquery lang-name query_src) -> (or TSQuery? bool?)
;; ```
;; * lang-name : string?
;; * query_src : string?
(define string->tsquery helix.ts.string->tsquery)

(provide query-tssyntax-byte-range)
;;@doc
;; Run a query on the given TSSyntax parse tree with a byte range
;; ```scheme
;; (query-tssyntax-byte-range query-loader syntax text lower upper) -> TSMatch?
;; ```
;; * query-loader : TSQueryLoader?
;; * syntax : TSSyntax?
;; * text : Rope?
;; * lower : (and int? positive?)
;; * upper : (and int? positive?)
(define query-tssyntax-byte-range helix.ts.query-tssyntax-byte-range)

(provide query-tssyntax)
;;@doc
;; Run a treesitter query on a given document's parse tree
;; ```scheme
;; (query-tssyntax query-loader syntax text) -> TSMatch?
;; ```
;; * query : TSQueryLoader?
;; * syntax : TSSyntax?
;; * text : Rope?
(define query-tssyntax helix.ts.query-tssyntax)

(provide rope->tssyntax)
;;@doc
;; Parse the syntax tree from given a language name and source
;; ```scheme
;; (rope->tssyntax src lang) -> (or TSQuery? bool?)
;; ```
;; * src : Rope?
;; * lang : string?
(define rope->tssyntax helix.ts.rope->tssyntax)
