[
  "Output"
  "Backspace"
  "Down"
  "Enter"
  "Escape"
  "Left"
  "Right"
  "Space"
  "Tab"
  "Up"
  "Set"
  "Type"
  "Sleep"
  "Hide"
  "Show" ] @keyword

[ "Shell"
  "FontFamily"
  "FontSize"
  "Framerate"
  "PlaybackSpeed"
  "Height"
  "LetterSpacing"
  "TypingSpeed"
  "LineHeight"
  "Padding"
  "Theme"
  "LoopOffset"
  "Width"
  "BorderRadius"
  "Margin"
  "MarginFill"
  "WindowBar"
  "WindowBarSize"
  "CursorBlink" ] @type

[ "@" ] @operator
(control) @function.macro
(float) @constant.numeric.float
(integer) @constant.numeric.integer
(comment) @comment
[(string) (json)] @string.special.path
(path) @string.special.path
(time) @string.special.symbol
(boolean) @constant.builtin.boolean
