# Hard wrap at input time (vim `textwidth`)

Status: design (decided), phase 1 implemented (2026-10-10). Upstream tracking issue:
[helix#2274](https://github.com/helix-editor/helix/issues/2274).

Goal: optionally hard-wrap lines as the user types, breaking newly entered text that exceeds
`text-width` onto the next line — like vim's `textwidth` or Emacs auto-fill. Hard wrap writes real
newlines into the buffer for the lines being edited only. Soft wrap remains an independent,
display-only transform.

## Current state

- **Piece**: Soft wrap
  - **Location**: `helix-core/src/doc_formatter.rs` (`DocumentFormatter`)
  - **Relevance**: Display-only word-wrap traversal yielding graphemes + visual coords; tabs/wide
    chars/indent-aware. Notably: `wrap_word` indent carry-over uses the col of the first
    *whitespace-excluding* grapheme (:405), and `Grapheme::Newline` counts width 1
    (`graphemes.rs:62`).
- **Piece**: `:reflow`
  - **Location**: `typed.rs:2480` → `helix-core/src/wrap.rs`
  - **Relevance**: `textwrap::refill` = **optimal-fit**: joins the selection into a paragraph and
    re-splits globally. Non-local rebalancing is explicitly unwanted while typing (upstream
    consensus, incl. textwrap author). Also miscounts tab width (no `tab_width` expansion).
- **Piece**: Width
  - **Location**: `Document::text_width()` (`document.rs:2344`)
  - **Relevance**: editorconfig `max-line-length` → `languages.toml` `text-width` →
    `editor.text-width` (80). Shared by `:reflow` and `soft-wrap.wrap-at-text-width`.
- **Piece**: Insert path
  - **Location**: `insert::insert_char` (`commands.rs:4351`)
  - **Relevance**: One `Transaction` for all ranges; `PostInsertChar` event dispatched after apply
    (`events.rs:13`), already consumed by completion/signature-help.
- **Piece**: Comment continuation
  - **Location**: `continued_line_comment_token` (`commands.rs:3833`), gated by `continue-comments`
  - **Relevance**: Syntax-layer-aware (handles injections), longest-token match via
    `comment::get_comment_token`. Used by Enter (`insert_newline`) and `o`/`O`. Continuation text =
    indent + token + `" "`.
- **Piece**: Indent
  - **Location**: `indent.rs`: `indent_for_newline` (:1277), `treesitter_indent_for_pos` (:978)
  - **Relevance**: Tree-sitter indent with `@opaque` nodes preserving leading whitespace (:1020).
    Silent whitespace-level fallback when no query exists (:1366).
- **Piece**: Undo
  - **Location**: `document.rs:1663`
  - **Relevance**: Successive `apply()`s compose into one pending changeset → a second "wrap"
    transaction is still one undo step (but emits a 2nd LSP didChange unless composed).

Query coverage (verified):
**markdown, gitcommit, tex, typst, rst, mail, adoc have no `indents.scm`**; only code languages ship
one. `@opaque` covers string literals, not comments.

## Agreed semantics

- **No mid-word splitting, ever.** Long words/tokens/URLs overflow until a space is typed past them;
  refinement is the formatter/LSP's job. (Softwrap's forced mid-word wrapping via `max-wrap` must
  not leak into hard wrap.)
- **Trigger:** vim rule — break when a *non-whitespace* char is inserted past the margin.
  (Alternative considered: emacs, fill on space/Enter whenever over width.)
- **Scope:** only rope lines touched by the current edit. Pre-existing long lines are left alone
  until edited.
- **Backspace join = escape hatch.** Wrapping is insert-triggered, so deleting a wrap newline joins
  the lines into one over-width line that stays as-is until the next insert past the margin (vim
  behavior). Manual un-wrapping just works.
- **Paste, completion, snippets:** out of scope initially (vim disables `textwidth` while pasting).
- **Continuation prefix** inserted at the break:
  `"\n" + indent (+ list marker) (+ comment token + " ")`.
  - Comment leaders reuse `continue-comments` + `continued_line_comment_token`.
  - Markdown list items must align under *content* (`- foo` → continuation at col 2), not the marker
    and not col 0. Col 0 relies on CommonMark lazy continuation (fragile); repeating the marker
    would start a second list item.
  - **Width budget:** first line keeps `text_width` (its prefix is real text); continuation rows get
    `text_width − display_width(prefix)`. If a single word + prefix doesn't fit, don't wrap.
- **Soft wrap untouched.** The `wrap-at-text-width` display agrees with hard wrap only where the
  continuation prefix is pure whitespace: softwrap carries just the leading-whitespace column
  (:405), so on comment/list lines its idea of the break point can differ from hard wrap's by up to
  `display_width(prefix)`. Every line hard wrap *writes* fits within `text_width` and renders
  unwrapped regardless; the divergence only concerns where softwrap would have broken the line had
  it stayed whole (see Option A).

## Continuation-prefix resolver (shared by both options)

Precedence, mirroring `continued_line_comment_token`'s layer-then-fallback pattern:

1. Base = line's leading whitespace (floor; full answer for plain text).
2. If the language (or injection layer) has an indent query: call `treesitter_indent_for_pos`
   **directly** (source-visible, unlike `indent_for_newline`'s silent fallback);
   `base = max(ts, base)` — scope indent can add structure but never pull a line below its own base.
   (As built this calls `indent_for_newline` wholesale instead — see the Phase-1 notes.)
   The `max` is the safety valve for the main TS/hard-wrap question mismatch: TS indent answers "new
   *code* line here", hard wrap wants "this line's content column" (e.g. deliberately outdented
   `// ---- section ----` banners inside indented blocks). Follow-up: mark comments `@opaque`
   upstream.
3. Line-local list-marker heuristic on top (no current grammar models prose lists):
   `^(\s*(?:[-*+]|\d+[.)])\s+)` → `base = max(base, leading_ws + marker_width)` — max, not add, so
   it cannot double-count if a grammar later models lists itself. Idempotent on our own continuation
   lines. Conservative gating (marker followed by space, bounded prefix width) so `---`, `x = -y`
   don't match. The heuristic is language-gated (config below): enablement is resolved per position
   via the same `layers_for_byte_range` walk as comment tokens, so a markdown-injected doc comment
   (`/// - item` via the rustdoc injection) enables it even under a code root language; grammar-less
   docs fall back to the editor-level default.
4. Comment token + `" "` if `continue-comments` applies.

TS-primary is the right architecture, but with today's query coverage the heuristic does the real
work for every prose format. Companion PRs that give "primary" substance: a markdown `indents.scm`
(list_item content alignment — also fixes Enter-in-lists, which currently produce col 0) and
`@opaque` for comments in code grammars. TS also buys Enter-parity where queries exist, and
injection handling comes for free.

## Option A — wrap-point via the softwrap traversal

Maintainer direction in #2274 (pascalkuthe): use the softwrap infrastructure; "We only want to have
a single wrapping implementation"; eventually rewrite `:reflow` on it.

Mechanics: on trigger, traverse the current line with `DocumentFormatter::new_at_prev_checkpoint`
under `TextFormat { soft_wrap: true, viewport_width: text_width, wrap_indicator: "" }` and empty
annotations (as built uses `viewport_width: text_width + 1` — see the Phase-1 notes). The first
document grapheme with `visual_pos.row > 0` (after skipping virtual text) is the wrap candidate →
insert break + continuation prefix at its `char_idx` (absorbing the preceding space). Then iterate:
with the prefix now consuming continuation-row width, re-check; if the moved segment still overflows,
break one word earlier; repeat (typically 1–2 iterations/keystroke).

Pros: one wrap implementation; wrap points identical to `wrap-at-text-width` display;
tab/CJK/word-boundary handling inherited; line-local, cheap.

Cons / risks: needs a new wrap-point helper — `DocumentFormatter` is a display iterator with no such
API; quirks to handle: phantom EOF grapheme (width 1, :276), `Newline` counted as width 1,
wrap-indicator splicing, virtual annotations. The prefix width cannot flow through one traversal
pass (row 0 and row ≥1 have equal budgets), hence the iteration loop. Mid-typing trees are
incomplete — resolver must be stateless/idempotent. Render-path isolation: the helper is an additive
*consumer* of `DocumentFormatter`; the softwrap render path is only at risk if shared internals
change (e.g. `Newline` width or `indent_level` semantics), which this design deliberately avoids —
continuation prefixes are computed outside the traversal. The existing softwrap test suite
(`doc_formatter/test.rs`, `position.rs`) guards refactor regressions.

Estimate: doc_formatter wrap-point helper + TextFormat setup ~100–150; resolver ~120–180;
trigger/transaction/driver ~100–150; config+docs ~50; tests ~100–150. **≈ 470–680 lines.**

## Option C — vim-style break at last space

Mechanics: on trigger, measure the line's display width (existing grapheme/width utils in
`helix-core::graphemes` / `position.rs`). Scan spaces left-to-right with the two-phase budget (full
width, then `text_width − prefix_width`); the last space at/before the budget becomes
`"\n" + continuation prefix`. No break found → do nothing (long words overflow by design).

Pros: exactly vim/emacs semantics; trivially predictable; cheapest per keystroke; prefix + budget
fold into the scan in one pass; no interaction with display machinery quirks.

Cons: a third break-point implementation, divergent from what softwrap displays for long tokens; no
reuse of the softwrap core the maintainers want to converge on.

Estimate: width/scan helper in helix-core ~80–120; resolver ~120–180; trigger/transaction/driver
~80–120; config+docs ~50; tests ~80–120. **≈ 410–590 lines.**

## Final design — Option A

Decided: go straight to **A** — it matches maintainer direction and keeps a single wrapping
implementation. A is not meaningfully more code than C: the engines differ by only ~30–90 lines
(wrap-point helper vs space scan) on top of a shared ~350–450 lines (resolver, driver, config,
tests); A's premium is the softwrap-quirks handling and its integration risk (bounded, see
render-path isolation above), while C touches `doc_formatter.rs` not at all. (C was considered as a
shippable first cut; since the resolver and config are option-agnostic, an engine swap would have
remained possible either way.)

## Config surface

`list-markers` follows the `auto-pairs` precedent (`AutoPairConfig`, `syntax/config.rs:536`): an
untagged enum in the same spirit — a bare bool enables/disables the built-in marker set; a bare
string overrides the marker regex.

```toml
[editor.hard-wrap]
enable = false    # width reuses the text-width chain
list-markers = true
# or a custom marker regex (must match the full prefix up to the content column):
# list-markers = '^(\s*(?:[-*+•]|\d+[.)])\s+)'
```

Like `auto-pairs`: `false` disables the heuristic, `true` uses the built-in set (`[-*+]`, `N.` /
`N)`), and a string replaces the pattern wholesale. Per-language overrides in `languages.toml` use
`Option<ListMarkersConfig>` with the usual fallback. The editor-level default ships as `true`
initially — unknown-type files are prose-likely, and the conservative gating (marker followed by
space, bounded prefix width) limits false positives. A partial `[editor.hard-wrap]` table keeps the
defaults for keys it omits (so `enable = true` alone leaves `list-markers = true`). Bundled
per-language defaults are still pending (see Deferred / backlog). Gated by the existing
`continue-comments` for comment leaders; orthogonal to `soft-wrap` and `rulers`.

## Implementation phasing & tracking

1. **Phase 1 — implemented.** Config gate + continuation-prefix resolver + break engine (A's
   softwrap traversal) behind `[editor.hard-wrap]`, triggered from `insert_char`. Typing with the
   feature enabled hard-wraps inserted text (incl. auto-pairs chars; multi-cursor deduped per line);
   pasted text, completion/snippet inserts, and Enter are unchanged.
  Implementation notes:
  - The wrap is a second transaction composing into the same pending changeset (one undo step);
   wrapped keystrokes emit an extra LSP didChange.
  - Prefix = `indent(+tree-sitter) + comment token + marker spaces`: the marker is matched after
    indent+comment (so `// - foo` aligns) and contributes its display width as plain spaces, never a
    repeated marker. The TS step calls `indent_for_newline` wholesale — its silent whitespace
    fallback is neutralized by the width-floor, and Hybrid blending + injection shifting come free.
  - Engine: viewport = `text_width + 1`, `soft_wrap_at_text_width` off (compensates softwrap's
    exact-fit relocation quirk); `point == 0` never wraps (single overlong first word).
  - Marker digits bounded to 3. `enable` and `list-markers` are both resolved by
   `Document::hard_wrap_config_at` (innermost language layer → document language → global; a
   language-less doc reduces to the editor default). Tests: helix-core unit (engine, matcher);
   helix-term integration (7 cases incl. no-split, marker alignment and a custom-regex case).
2. Optional emacs-style trigger variant: break as soon as the line *exceeds* the width on any
   insertion (including the overflowing space itself — so no trailing-space overhang past the
   margin, unlike vim), and fill the text left of the cursor on Enter (Enter becomes "wrap
   remainder, then newline"). Plus multi-cursor dedupe test coverage and optional paste/completion
   handling.

## Deferred / backlog

- Enter-parity: adopt the resolver in the `indent_for_newline` fallback so Enter in markdown lists
  also aligns content — consistency between the Enter and hard-wrap paths matters.
- Tests that need a language-matched buffer: comment-token continuation and multi-cursor same-line
  dedupe (logic in place, not covered end-to-end).
- Custom `list-markers` regex is compiled (and its pattern cloned by the resolver) for every wrapped
  line, not once per trigger; cache it if it ever shows up hot.
- Banner-comment misalignment in code languages once comments are `@opaque`.
- Bundled `languages.toml` overrides for prose languages (markdown, gitcommit, mail, tex, ...) and a
  grammar-less `text` entry (`*.txt`, ...). The editor-level `list-markers` default stays necessary
  regardless: `file-types` globs mean formats with their own grammars never map to `text`, and files
  matching nothing stay language-less and rely on the editor-level default.
- Companion upstream work: a markdown `indents.scm` and the eventual `:reflow` rewrite on the
  softwrap core.
