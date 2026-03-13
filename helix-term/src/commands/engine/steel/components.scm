;; Note: This does not include all of the bindings. This is just a template.
;; More bindings are added at runtime for registering key events.

(require-builtin helix/components as helix.components.)
(provide async-read-line)
(define async-read-line helix.components.async-read-line)

(provide make-async-reader-writer)
(define make-async-reader-writer helix.components.make-async-reader-writer)

(provide theme->bg)
;;@doc
;; DEPRECATED: Please use `theme-scope`
;;Gets the `Style` associated with the bg for the current theme
(define theme->bg helix.components.theme->bg)

(provide theme->fg)
;;@doc
;; DEPRECATED: Please use `theme-scope`
;;Gets the `style` associated with the fg for the current theme
(define theme->fg helix.components.theme->fg)

(provide theme-scope-ref)
;;@doc
;;Get the `Style` associated with the given scope from the current theme
;; ```
;; (theme-scope-ref scope)
;; ```
;; scope : string?
;;
;; # Examples
;;
;; ```scheme
;; (theme-scope-ref "ui.text")
;; ```
(define theme-scope-ref helix.components.theme-scope-ref)

(provide theme-scope)
;;@doc
;; Note: Prefer using theme-scope-ref. This is left in for backwards compatibility.
;;
;;Get the `Style` associated with the given scope from the current theme
;; ```
;; (theme-scope scope)
;; ```
;; scope : string?
;;
;; # Examples
;;
;; ```scheme
;; (theme-scope "ui.text")
;; ```
(define (theme-scope scope)
  (helix.components.theme-scope *helix.cx* scope))

(provide Position?)
;;@doc
;;Check if the given value is a `Position`
;;
;;```scheme
;;(Position? value) -> bool?
;;```
;;
;;value : any?
;;
;;
(define Position? helix.components.Position?)

(provide Style?)
;;@doc
;;Check if the given valuie is `Style`
;;
;;```scheme
;;(Style? value) -> bool?
;;```
;;
;;value : any?
(define Style? helix.components.Style?)

(provide Buffer?)
;;@doc
;;
;;Checks if the given value is a `Buffer`
;;
;;```scheme
;;(Buffer? value) -> bool?
;;```
;;
;;value : any?
;;
(define Buffer? helix.components.Buffer?)

(provide buffer-area)
;;@doc
;;
;;Get the `Rect` associated with the given `Buffer`
;;
;;```scheme
;;(buffer-area buffer)
;;```
;;
;;* buffer : Buffer?
;;
(define buffer-area helix.components.buffer-area)

(provide frame-set-string!)
;;@doc
;;
;;Set the string at the given `x` and `y` positions for the given `Buffer`, with a provided `Style`.
;;
;;```scheme
;;(frame-set-string! buffer x y string style)
;;```
;;
;;buffer : Buffer?,
;;x : int?,
;;y : int?,
;;string: string?,
;;style: Style?,
;;
(define frame-set-string! helix.components.frame-set-string!)

(provide SteelEventResult?)
;;@doc
;;
;;Check whether the given value is a `SteelEventResult`.
;;
;;```scheme
;;(SteelEventResult? value) -> bool?
;;```
;;
;;value : any?
;;
;;
(define SteelEventResult? helix.components.SteelEventResult?)

(provide new-component!)
;;@doc
;;
;;Construct a new dynamic component. This is used for creating widgets or floating windows
;;that exist outside of the buffer. This just constructs the component, it does not push the component
;;on to the component stack. For that, you'll use `push-component!`.
;;
;;```scheme
;;(new-component! name state render function-map)
;;```
;;
;;name : string? - This is the name of the comoponent itself.
;;state : any? - Typically this is a struct that holds the state of the component.
;;render : (-> state? Rect? Buffer?)
;;    This is a function that will get called with each frame. The first argument is the state object provided,
;;    and the second is the `Rect?` to render against, ultimately against the `Buffer?`.
;;
;;function-map : (hashof string? function?)
;;    This is a hashmap of strings -> function that contains a few important functions:
;;
;;    "handle_event" : (-> state? Event?) -> SteelEventResult?
;;
;;        This is called on every event with an event object. There are multiple options you can use
;;        when returning from this function:
;;
;;        * event-result/consume
;;        * event-result/consume-without-rerender
;;        * event-result/ignore
;;        * event-result/close
;;
;;        See the associated docs for those to understand the implications for each.
;;
;;    "cursor" : (-> state? Rect?) -> Position?
;;
;;        This tells helix where to put the cursor.
;;
;;    "required_size": (-> state? (pair? int?)) -> (pair? int?)
;;
;;        Seldom used: TODO
;;
(define new-component! helix.components.new-component!)

(provide position)
;;@doc
;;
;;Construct a new `Position`.
;;
;;```scheme
;;(position row col) -> Position?
;;```
;;
;;row : int?
;;col : int?
;;
(define position helix.components.position)

(provide position-row)
;;@doc
;;
;;Get the row associated with the given `Position`.
;;
;;```scheme
;;(position-row pos) -> int?
;;```
;;
;;pos : `Position?`
;;
(define position-row helix.components.position-row)

(provide position-col)
;;@doc
;;
;;Get the col associated with the given `Position`.
;;
;;```scheme
;;(position-col pos) -> int?
;;```
;;
;;pos : `Position?`
(define position-col helix.components.position-col)

(provide set-position-row!)
;;@doc
;;Set the row for the given `Position`
;;
;;```scheme
;;(set-position-row! pos row)
;;```
;;
;;pos : Position?
;;row : int?
;;
(define set-position-row! helix.components.set-position-row!)

(provide set-position-col!)
;;@doc
;;Set the col for the given `Position`
;;
;;```scheme
;;(set-position-col! pos col)
;;```
;;
;;pos : Position?
;;col : int?
;;
(define set-position-col! helix.components.set-position-col!)

(provide Rect?)
;;@doc
;;Check if the given value is a `Rect`
;;
;;```scheme
;;(Rect? value) -> bool?
;;```
;;
;;value : any?
;;
;;
(define Rect? helix.components.Rect?)

(provide area)
;;@doc
;;
;;Constructs a new `Rect`.
;;
;;(area x y width height)
;;
;;* x : int?
;;* y : int?
;;* width: int?
;;* height: int?
;;
;;# Examples
;;
;;```scheme
;;(area 0 0 100 200)
;;```
(define area helix.components.area)

(provide area-x)
;;@doc
;;Get the `x` value of the given `Rect`
;;
;;```scheme
;;(area-x area) -> int?
;;```
;;
;;area : Rect?
;;
(define area-x helix.components.area-x)

(provide area-y)
;;@doc
;;Get the `y` value of the given `Rect`
;;
;;```scheme
;;(area-y area) -> int?
;;```
;;
;;area : Rect?
;;
(define area-y helix.components.area-y)

(provide area-width)
;;@doc
;;Get the `width` value of the given `Rect`
;;
;;```scheme
;;(area-width area) -> int?
;;```
;;
;;area : Rect?
;;
(define area-width helix.components.area-width)

(provide area-height)
;;@doc
;;Get the `height` value of the given `Rect`
;;
;;```scheme
;;(area-height area) -> int?
;;```
;;
;;area : Rect?
;;
(define area-height helix.components.area-height)

(provide native-component-required-size)
(define native-component-required-size helix.components.native-component-required-size)

(provide render-native-component)
;;@doc
;; Render a native component
(define (render-native-component component area buffer)
  (helix.components.render-native-component component area buffer *helix.cx*))

(provide markdown-component)
;;@doc
;; Render a native component
;;```
;; (markdown-component text)
;; ```
(define markdown-component helix.components.markdown-component)

(provide overlaid)
(define overlaid helix.components.overlaid)

(provide Widget/list?)
;;@doc
;;Check whether the given value is a list widget.
;;
;;```scheme
;;(Widget/list? value) -> bool?
;;```
;;
;;value : any?
;;
(define Widget/list? helix.components.Widget/list?)

(provide widget/list)
;;@doc
;;Creates a new `List` widget with the given items.
;;
;;```scheme
;;(widget/list lst) -> Widget?
;;```
;;
;;* lst : (listof string?)
;;
(define widget/list helix.components.widget/list)

(provide widget/list/render)
;;@doc
;;
;;
;;Render the given `Widget/list` onto the provided `Rect` within the given `Buffer`.
;;
;;```scheme
;;(widget/list/render buf area lst)
;;```
;;
;;* buf : `Buffer?`
;;* area : `Rect?`
;;* lst : `Widget/list?`
;;
(define widget/list/render helix.components.widget/list/render)

(provide block)
;;@doc
;;Creates a block with the following styling:
;;
;;```scheme
;;(block)
;;```
;;
;;* borders - all
;;* border-style - default style + white fg
;;* border-type - rounded
;;* style - default + black bg
;;
(define block helix.components.block)

(provide make-block)
;;@doc
;;
;;Create a `Block` with the provided styling, borders, and border type.
;;
;;
;;```scheme
;;(make-block style border-style borders border_type)
;;```
;;
;;* style : Style?
;;* border-style : Style?
;;* borders : string?
;;* border-type: String?
;;
;;Valid border-types include:
;;* "plain"
;;* "rounded"
;;* "double"
;;* "thick"
;;
;;Valid borders include:
;;* "top"
;;* "left"
;;* "right"
;;* "bottom"
;;* "all"
;;
(define make-block helix.components.make-block)

(provide block/render)
;;@doc
;;
;;Render the given `Block` over the given `Rect` onto the provided `Buffer`.
;;
;;```scheme
;;(block/render buf area block)
;;```
;;
;;buf : Buffer?
;;area: Rect?
;;block: Block?
;;
;;
(define block/render helix.components.block/render)

(provide buffer/clear)
;;@doc
;;Clear a `Rect` in the `Buffer`
;;
;;```scheme
;;(buffer/clear frame area)
;;```
;;frame : Buffer?
;;area : Rect?
;;
(define buffer/clear helix.components.buffer/clear)

(provide buffer/clear-with)
;;@doc
;;Clear a `Rect` in the `Buffer` with a default `Style`
;;
;;```scheme
;;(buffer/clear-with frame area style)
;;```
;;frame : Buffer?
;;area : Rect?
;;style : Style?
;;
(define buffer/clear-with helix.components.buffer/clear-with)

(provide set-color-rgb!)
;;@doc
;;
;;Mutate the r/g/b of a color in place, to avoid allocation.
;;
;;```scheme
;;(set-color-rgb! color r g b)
;;```
;;
;;color : Color?
;;r : int?
;;g : int?
;;b : int?
(define set-color-rgb! helix.components.set-color-rgb!)

(provide set-color-indexed!)
;;@doc
;;
;;Mutate this color to be an indexed color.
;;
;;```scheme
;;(set-color-indexed! color index)
;;```
;;
;;color : Color?
;;index: int?
;;
(define set-color-indexed! helix.components.set-color-indexed!)

(provide Color?)
;;@doc
;;Check if the given value is a `Color`.
;;
;;```scheme
;;(Color? value) -> bool?
;;```
;;
;;value : any?
;;
;;
(define Color? helix.components.Color?)

(provide Color/Reset)
;;@doc
;;
;;Singleton for the reset color.
;;
(define Color/Reset helix.components.Color/Reset)

(provide Color/Black)
;;@doc
;;
;;Singleton for the color black.
;;
(define Color/Black helix.components.Color/Black)

(provide Color/Red)
;;@doc
;;
;;Singleton for the color red.
;;
(define Color/Red helix.components.Color/Red)

(provide Color/White)
;;@doc
;;
;;Singleton for the color white.
;;
(define Color/White helix.components.Color/White)

(provide Color/Green)
;;@doc
;;
;;Singleton for the color green.
;;
(define Color/Green helix.components.Color/Green)

(provide Color/Yellow)
;;@doc
;;
;;Singleton for the color yellow.
;;
(define Color/Yellow helix.components.Color/Yellow)

(provide Color/Blue)
;;@doc
;;
;;Singleton for the color blue.
;;
(define Color/Blue helix.components.Color/Blue)

(provide Color/Magenta)
;;@doc
;;
;;Singleton for the color magenta.
;;
(define Color/Magenta helix.components.Color/Magenta)

(provide Color/Cyan)
;;@doc
;;
;;Singleton for the color cyan.
;;
(define Color/Cyan helix.components.Color/Cyan)

(provide Color/Gray)
;;@doc
;;
;;Singleton for the color gray.
;;
(define Color/Gray helix.components.Color/Gray)

(provide Color/LightRed)
;;@doc
;;
;;Singleton for the color light read.
;;
(define Color/LightRed helix.components.Color/LightRed)

(provide Color/LightGreen)
;;@doc
;;
;;Singleton for the color light green.
;;
(define Color/LightGreen helix.components.Color/LightGreen)

(provide Color/LightYellow)
;;@doc
;;
;;Singleton for the color light yellow.
;;
(define Color/LightYellow helix.components.Color/LightYellow)

(provide Color/LightBlue)
;;@doc
;;
;;Singleton for the color light blue.
;;
(define Color/LightBlue helix.components.Color/LightBlue)

(provide Color/LightMagenta)
;;@doc
;;
;;Singleton for the color light magenta.
;;
(define Color/LightMagenta helix.components.Color/LightMagenta)

(provide Color/LightCyan)
;;@doc
;;
;;Singleton for the color light cyan.
;;
(define Color/LightCyan helix.components.Color/LightCyan)

(provide Color/LightGray)
;;@doc
;;
;;Singleton for the color light gray.
;;
(define Color/LightGray helix.components.Color/LightGray)

(provide Color/rgb)
;;@doc
;;
;;Construct a new color via rgb.
;;
;;```scheme
;;(Color/rgb r g b) -> Color?
;;```
;;
;;r : int?
;;g : int?
;;b : int?
;;
(define Color/rgb helix.components.Color/rgb)

(provide Color-red)
;;@doc
;;
;;Get the red component of the `Color?`.
;;
;;```scheme
;;(Color-red color) -> int?
;;```
;;
;;color * Color?
;;
(define Color-red helix.components.Color-red)

(provide Color-green)
;;@doc
;;
;;Get the green component of the `Color?`.
;;
;;```scheme
;;(Color-green color) -> int?
;;```
;;
;;color * Color?
(define Color-green helix.components.Color-green)

(provide Color-blue)
;;@doc
;;
;;Get the blue component of the `Color?`.
;;
;;```scheme
;;(Color-blue color) -> int?
;;```
;;
;;color * Color?
(define Color-blue helix.components.Color-blue)

(provide Color/Indexed)
;;@doc
;;
;;
;;Construct a new indexed color.
;;
;;```scheme
;;(Color/Indexed index) -> Color?
;;```
;;
;;* index : int?
;;
(define Color/Indexed helix.components.Color/Indexed)

(provide set-style-fg!)
;;@doc
;;
;;
;;Mutates the given `Style` to have the fg with the provided color.
;;
;;```scheme
;;(set-style-fg! style color)
;;```
;;
;;style : `Style?`
;;color : `Color?`
;;
(define set-style-fg! helix.components.set-style-fg!)

(provide style-fg)
;;@doc
;;
;;
;;Constructs a new `Style` with the provided `Color` for the fg.
;;
;;```scheme
;;(style-fg style color) -> Style
;;```
;;
;;style : Style?
;;color: Color?
;;
(define style-fg helix.components.style-fg)

(provide style-bg)
;;@doc
;;
;;
;;Constructs a new `Style` with the provided `Color` for the bg.
;;
;;```scheme
;;(style-bg style color) -> Style
;;```
;;
;;style : Style?
;;color: Color?
;;
(define style-bg helix.components.style-bg)

(provide style-with-italics)
;;@doc
;;
;;
;;Constructs a new `Style` with italcs.
;;
;;```scheme
;;(style-with-italics style) -> Style
;;```
;;
;;style : Style?
;;
(define style-with-italics helix.components.style-with-italics)

(provide style-with-bold)
;;@doc
;;
;;
;;Constructs a new `Style` with bold styling.
;;
;;```scheme
;;(style-with-bold style) -> Style
;;```
;;
;;style : Style?
;;
(define style-with-bold helix.components.style-with-bold)

(provide style-with-dim)
;;@doc
;;
;;
;;Constructs a new `Style` with dim styling.
;;
;;```scheme
;;(style-with-dim style) -> Style
;;```
;;
;;style : Style?
;;
(define style-with-dim helix.components.style-with-dim)

(provide style-with-slow-blink)
;;@doc
;;
;;
;;Constructs a new `Style` with slow blink.
;;
;;```scheme
;;(style-with-slow-blink style) -> Style
;;```
;;
;;style : Style?
;;
(define style-with-slow-blink helix.components.style-with-slow-blink)

(provide style-with-rapid-blink)
;;@doc
;;
;;
;;Constructs a new `Style` with rapid blink.
;;
;;```scheme
;;(style-with-rapid-blink style) -> Style
;;```
;;
;;style : Style?
;;
(define style-with-rapid-blink helix.components.style-with-rapid-blink)

(provide style-with-reversed)
;;@doc
;;
;;
;;Constructs a new `Style` with revered styling.
;;
;;```scheme
;;(style-with-reversed style) -> Style
;;```
;;
;;style : Style?
;;
(define style-with-reversed helix.components.style-with-reversed)

(provide style-with-hidden)
;;@doc
;;
;;Constructs a new `Style` with hidden styling.
;;
;;```scheme
;;(style-with-hidden style) -> Style
;;```
;;
;;style : Style?
;;
(define style-with-hidden helix.components.style-with-hidden)

(provide style-with-crossed-out)
;;@doc
;;
;;
;;Constructs a new `Style` with crossed out styling.
;;
;;```scheme
;;(style-with-crossed-out style) -> Style
;;```
;;
;;style : Style?
;;
(define style-with-crossed-out helix.components.style-with-crossed-out)

(provide style->fg)
;;@doc
;;
;;
;;Return the color on the style, or #false if not present.
;;
;;```scheme
;;(style->fg style) -> (or Color? #false)
;;```
;;
;;style : Style?
;;
;;
(define style->fg helix.components.style->fg)

(provide style->bg)
;;@doc
;;
;;
;;Return the color on the style, or #false if not present.
;;
;;```scheme
;;(style->bg style) -> (or Color? #false)
;;```
;;
;;style : Style?
;;
;;
(define style->bg helix.components.style->bg)

(provide set-style-bg!)
;;@doc
;;
;;
;;Mutate the background style on the given style to a given color.
;;
;;```scheme
;;(set-style-bg! style color)
;;```
;;
;;style : Style?
;;color : Color?
;;
;;
(define set-style-bg! helix.components.set-style-bg!)

(provide style-underline-color)
;;@doc
;;
;;
;;Return a new style with the provided underline color.
;;
;;```scheme
;;(style-underline-color style color) -> Style?
;;
;;```
;;style : Style?
;;color : Color?
;;
;;
(define style-underline-color helix.components.style-underline-color)

(provide style-underline-style)
;;@doc
;;
;;Return a new style with the provided underline style.
;;
;;```scheme
;;(style-underline-style style underline-style) -> Style?
;;
;;```
;;
;;style : Style?
;;underline-style : UnderlineStyle?
;;
(define style-underline-style helix.components.style-underline-style)

(provide UnderlineStyle?)
;;@doc
;;
;;Check if the provided value is an `UnderlineStyle`.
;;
;;```scheme
;;(UnderlineStyle? value) -> bool?
;;
;;```
;;value : any?
(define UnderlineStyle? helix.components.UnderlineStyle?)

(provide Underline/Reset)
;;@doc
;;
;;Singleton for resetting the underling style.
;;
(define Underline/Reset helix.components.Underline/Reset)

(provide Underline/Line)
;;@doc
;;
;;Singleton for the line underline style.
;;
(define Underline/Line helix.components.Underline/Line)

(provide Underline/Curl)
;;@doc
;;
;;Singleton for the curl underline style.
;;
(define Underline/Curl helix.components.Underline/Curl)

(provide Underline/Dotted)
;;@doc
;;
;;Singleton for the dotted underline style.
;;
(define Underline/Dotted helix.components.Underline/Dotted)

(provide Underline/Dashed)
;;@doc
;;
;;Singleton for the dashed underline style.
;;
(define Underline/Dashed helix.components.Underline/Dashed)

(provide Underline/DoubleLine)
;;@doc
;;
;;Singleton for the double line underline style.
;;
(define Underline/DoubleLine helix.components.Underline/DoubleLine)

(provide event-result/consume)
;;@doc
;;
;;Singleton for consuming an event. If this is returned from an event handler, the event
;;will not continue to be propagated down the component stack. This also will trigger a
;;re-render.
;;
(define event-result/consume helix.components.event-result/consume)

(provide event-result/consume-without-rerender)
;;@doc
;;
;;Singleton for consuming an event. If this is returned from an event handler, the event
;;will not continue to be propagated down the component stack. This will _not_ trigger
;;a re-render.
;;
(define event-result/consume-without-rerender helix.components.event-result/consume-without-rerender)

(provide event-result/ignore)
;;@doc
;;
;;Singleton for ignoring an event. If this is returned from an event handler, the event
;;will not continue to be propagated down the component stack. This will _not_ trigger
;;a re-render.
;;
(define event-result/ignore helix.components.event-result/ignore)

(provide event-result/ignore-and-close)
;;@doc
;;
;;Singleton for ignoring an event. If this is returned from an event handler, the event
;;will continue to be propagated down the component stack, and the component will be
;;popped off of the stack and removed.
;;
(define event-result/ignore-and-close helix.components.event-result/ignore-and-close)

(provide event-result/close)
;;@doc
;;
;;Singleton for consuming an event. If this is returned from an event handler, the event
;;will not continue to be propagated down the component stack, and the component will
;;be popped off of the stack and removed.
;;
(define event-result/close helix.components.event-result/close)

(provide style)
;;@doc
;;
;;Constructs a new default style.
;;
;;```scheme
;;(style) -> Style?
;;```
;;
(define style helix.components.style)

(provide Event?)
;;@doc
;;Check if this value is an `Event`
;;
;;```scheme
;;(Event? value) -> bool?
;;```
;;value : any?
;;
(define Event? helix.components.Event?)

(provide paste-event?)
;;@doc
;;Checks if the given event is a paste event.
;;
;;```scheme
;;(paste-event? event) -> bool?
;;```
;;
;;* event : Event?
;;
;;
(define paste-event? helix.components.paste-event?)

(provide paste-event-string)
;;@doc
;;Get the string from the paste event, if it is a paste event.
;;
;;```scheme
;;(paste-event-string event) -> (or string? #false)
;;```
;;
;;* event : Event?
;;
;;
(define paste-event-string helix.components.paste-event-string)

(provide key-event?)
;;@doc
;;Checks if the given event is a key event.
;;
;;```scheme
;;(key-event? event) -> bool?
;;```
;;
;;* event : Event?
;;
(define key-event? helix.components.key-event?)

(provide string->key-event)
;;@doc
;;Get a key event from a string
(define string->key-event helix.components.string->key-event)

(provide event->key-event)
;;@doc
;;Return the key event from an event, if it is one
(define event->key-event helix.components.event->key-event)

(provide key-event-char)
;;@doc
;;Get the character off of the event, if there is one.
;;
;;```scheme
;;(key-event-char event) -> (or char? #false)
;;```
;;event : Event?
;;
(define key-event-char helix.components.key-event-char)

(provide on-key-event-char)
;;@doc
;;Get the character off of the key event, if there is one.
;;
;;```scheme
;;(on-key-event-char event) -> (or char? #false)
;;```
;;event : KeyEvent?
;;
(define on-key-event-char helix.components.on-key-event-char)

(provide key-event-modifier)
;;@doc
;;
;;Get the key event modifier off of the event, if there is one.
;;
;;```scheme
;;(key-event-modifier event) -> (or int? #false)
;;```
;;event : Event?
;;
(define key-event-modifier helix.components.key-event-modifier)

(provide key-modifier-ctrl)
;;@doc
;;
;;The key modifier bits associated with the ctrl key modifier.
;;
(define key-modifier-ctrl helix.components.key-modifier-ctrl)

(provide key-modifier-shift)
;;@doc
;;
;;The key modifier bits associated with the shift key modifier.
;;
(define key-modifier-shift helix.components.key-modifier-shift)

(provide key-modifier-alt)
;;@doc
;;
;;The key modifier bits associated with the alt key modifier.
;;
(define key-modifier-alt helix.components.key-modifier-alt)

(provide key-modifier-super)
;;@doc
;;
;;The key modifier bits associated with the super key modifier.
;;
(define key-modifier-super helix.components.key-modifier-super)

(provide key-event-F?)
;;@doc
;;Check if this key event is associated with an `F<x>` key, e.g. F1, F2, etc.
;;
;;```scheme
;;(key-event-F? event number) -> bool?
;;```
;;event : Event?
;;number : int?
;;
(define key-event-F? helix.components.key-event-F?)

(provide mouse-event?)
;;@doc
;;
;;Check if this event is a mouse event.
;;
;;```scheme
;;(mouse-event event) -> bool?
;;```
;;event : Event?
(define mouse-event? helix.components.mouse-event?)

(provide event-mouse-kind)
;;@doc
;;Convert the mouse event kind into an integer representing the state.
;;
;;```scheme
;;(event-mouse-kind event) -> (or int? #false)
;;```
;;
;;event : Event?
;;
;;This is the current mapping today:
;;
;;```rust
;;match kind {
;;    helix_view::input::MouseEventKind::Down(MouseButton::Left) => 0,
;;    helix_view::input::MouseEventKind::Down(MouseButton::Right) => 1,
;;    helix_view::input::MouseEventKind::Down(MouseButton::Middle) => 2,
;;    helix_view::input::MouseEventKind::Up(MouseButton::Left) => 3,
;;    helix_view::input::MouseEventKind::Up(MouseButton::Right) => 4,
;;    helix_view::input::MouseEventKind::Up(MouseButton::Middle) => 5,
;;    helix_view::input::MouseEventKind::Drag(MouseButton::Left) => 6,
;;    helix_view::input::MouseEventKind::Drag(MouseButton::Right) => 7,
;;    helix_view::input::MouseEventKind::Drag(MouseButton::Middle) => 8,
;;    helix_view::input::MouseEventKind::Moved => 9,
;;    helix_view::input::MouseEventKind::ScrollDown => 10,
;;    helix_view::input::MouseEventKind::ScrollUp => 11,
;;    helix_view::input::MouseEventKind::ScrollLeft => 12,
;;    helix_view::input::MouseEventKind::ScrollRight => 13,
;;}
;;```
;;
;;Any unhandled event that does not match this will return `#false`.
(define event-mouse-kind helix.components.event-mouse-kind)

(provide event-mouse-row)
;;@doc
;;
;;
;;Get the row from the mouse event, of #false if it isn't a mouse event.
;;
;;```scheme
;;(event-mouse-row event) -> (or int? #false)
;;```
;;
;;event : Event?
;;
;;
(define event-mouse-row helix.components.event-mouse-row)

(provide event-mouse-col)
;;@doc
;;
;;
;;Get the col from the mouse event, of #false if it isn't a mouse event.
;;
;;```scheme
;;(event-mouse-row event) -> (or int? #false)
;;```
;;
;;event : Event?
;;
(define event-mouse-col helix.components.event-mouse-col)

(provide mouse-event-within-area?)
;;@doc
;;Check whether the given mouse event occurred within a given `Rect`.
;;
;;```scheme
;;(mouse-event-within-area? event area) -> bool?
;;```
;;
;;event : Event?
;;area : Rect?
;;
(define mouse-event-within-area? helix.components.mouse-event-within-area?)
