# You can move it here ~/.config/elvish/lib/hx.elv
# Or add `eval (slurp < ~/$REPOS/helix/contrib/completion/hx.elv)`
# Be sure to replace `$REPOS` with something that makes sense for you!

### Renders a pretty completion candidate
var candidate = { | _stem  _desc | 
  edit:complex-candidate $_stem &display=(styled $_stem bold)(styled " "$_desc dim)
}

### These commands will invalidate further input (i.e. not react to them)
var skips = [ "--tutor" "--help" "--version" "-V" "--health" ]

### Grammar commands
var grammar = [ "--grammar" "-g" ]

### Config commands
var config = [ "--config" "-c" ]

### Set an arg-completer for the `hx` binary
set edit:completion:arg-completer[hx] = {|@args|
  var n = (count $args)
  if (>= $n 3) {
    # Stop completions if passed arg will take presedence
    # and invalidate further input
    if (has-value $skips $args[-2]) {
      return
    } 
    # If the previous arg == --grammar, then only suggest:
    if (has-value $grammar $args[-2]) {
      $candidate "fetch" "Fetch the tree-sitter grammars"
      $candidate "build" "Build the tree-sitter grammars"
      return
    } 
    # When we have --config, we need a file
    if (has-values $config $args[-2]) {
      edit:complete-filename $args[-1] | each { |v| put $v[stem] }
      return
    } 
    # When we have --log, we need a file
    if (has-values "log" $args[-2]) {
      edit:complete-filename $args[-1] | each { |v| put $v[stem] }
      return
    } 
  } 
  edit:complete-filename $args[-1] | each { |v| put $v[stem]}
  $candidate "--help" "(Prints help information)"
  $candidate "--version" "(Prints version information)"
  $candidate "--tutor" "(Loads the tutorial)"
  $candidate "--health" "(Checks for errors in editor setup)"
  $candidate "--grammar" "(Fetch or build the tree-sitter grammars)"
  $candidate "--vsplit" "(Splits all given files vertically)"
  $candidate "--hsplit" "(Splits all given files horizontally)"
  $candidate "--config" "(Specifies a file to use for configuration)"
  $candidate "--log" "(Specifies a file to write log data into)"
}
