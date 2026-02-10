#!/usr/bin/env fish
# Fish completion script for Silicon editor

complete -c si -s h -l help -d "Prints help information"
complete -c si -l tutor -d "Loads the tutorial"
complete -c si -l health -xa "(__hx_langs_ops)" -d "Checks for errors"
complete -c si -l health -xka all -d "Prints all diagnostic informations"
complete -c si -l health -xka all-languages -d "Lists all languages"
complete -c si -l health -xka languages -d "Lists user configured languages"
complete -c si -l health -xka clipboard -d "Prints system clipboard provider"
complete -c si -s g -l grammar -x -a "fetch build" -d "Fetch or build tree-sitter grammars"
complete -c si -s v -o vv -o vvv -d "Increases logging verbosity"
complete -c si -s V -l version -d "Prints version information"
complete -c si -l vsplit -d "Splits all given files vertically"
complete -c si -l hsplit -d "Splits all given files horizontally"
complete -c si -s c -l config -r -d "Specifies a file to use for config"
complete -c si -l log -r -d "Specifies a file to use for logging"
complete -c si -s w -l working-dir -d "Specify initial working directory" -xa "(__fish_complete_directories)"

function __hx_langs_ops
    si --health all-languages | tail -n '+2' | string replace -fr '^(\S+) .*' '$1'
end
