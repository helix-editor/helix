#!/usr/bin/env bash
# Bash completion script for Helix editor

_hx() {
    local cur prev languages line
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD - 1]}"

    case "$prev" in
    -g | --grammar)
        while IFS= read -r line; do COMPREPLY+=("$line"); done < <(compgen -W 'fetch build' -- "$cur")
        return 0
        ;;
    --health)
        languages=$(hx --health all-languages | tail -n '+2' | awk '{print $1}' | sed 's/\x1b\[[0-9;]*m//g')
        while IFS= read -r line; do COMPREPLY+=("$line"); done < <(compgen -W """clipboard languages all-languages all $languages""" -- "$cur")
        return 0
        ;;
    esac

    case "$2" in
    -*)
        while IFS= read -r line; do COMPREPLY+=("$line"); done < <(compgen -W "-h --help --strict --tutor -V --version -v -vv -vvv --health -g --grammar --vsplit --hsplit -c --config --log" -- """$2""")
        return 0
        ;;
    *)
        while IFS= read -r line; do COMPREPLY+=("$line"); done < <(compgen -fd -- """$2""")
        return 0
        ;;
    esac
} && complete -o filenames -F _hx hx
