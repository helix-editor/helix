#!/usr/bin/env fish
# Fish completion script for Helix editor

set -l langs (hx --health |tail -n '+7' |awk '{print $1}' |sed 's/\x1b\[[0-9;]*m//g')

complete -c hx -s h -l help -d "Prints help information"
complete -c hx -l tutor -d "Loads the tutorial"
complete -c hx -l health -x -a "$langs" -d "Checks for errors in editor setup"
complete -c hx -s g -l grammar -x -a "fetch build" -d "Fetches or builds tree-sitter grammars"
complete -c hx -s v -o vv -o vvv -d "Increases logging verbosity"
complete -c hx -s V -l version -d "Prints version information"
complete -c hx -l vsplit -d "Splits all given files vertically into different windows"
complete -c hx -l hsplit -d "Splits all given files horizontally into different windows"
