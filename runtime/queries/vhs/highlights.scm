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

[ "FontFamily"
  "FontSize"
  "Framerate"
  "Height"
  "LetterSpacing"
  "TypingSpeed"
  "LineHeight"
  "Padding"
  "Theme"
  "Width" ] @type

[ "@" ] @operator
(control) @function.macro
(float) @constant.numeric.float
(integer) @constant.numeric.integer
(comment) @comment
(path) @string.special.path
[(string) (json)] @string
(time) @string.special.symbol