#!/usr/bin/env fish
# Fish completion script for Helix editor

complete -c hx -s h -l help -d "Prints help information"
complete -c hx -l tutor -d "Loads the tutorial"
complete -c hx -l health -xa "(__hx_langs_ops)" -d "Checks for errors in editor setup"
complete -c hx -s g -l grammar -x -a "fetch build" -d "Fetch or build tree-sitter grammars"
complete -c hx -s v -o vv -o vvv -d "Increases logging verbosity"
complete -c hx -s V -l version -d "Prints version information"
complete -c hx -l vsplit -d "Splits all given files vertically"
complete -c hx -l hsplit -d "Splits all given files horizontally"
complete -c hx -s c -l config -r -d "Specifies a file to use for config"
complete -c hx -l log -r -d "Specifies a file to use for logging"
complete -c hx -s w -l working-dir -d "Specify initial working directory" -xa "(__fish_complete_directories)"

function __hx_langs_ops
    hx --health | tail -n '+7' | awk '{print $1}' | sed 's/\x1b\[[0-9;]*m//g'
end
