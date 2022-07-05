#!/usr/bin/env bash
# Bash completion script for Helix editor

_hx() {
	# $1 command name
	# $2 word being completed
	# $3 word preceding
	COMPREPLY=()

	case "$3" in
	-g | --grammar)
		COMPREPLY=($(compgen -W "fetch build" -- $2))
		;;
	--health)
		local languages=$(hx --health |tail -n '+7' |awk '{print $1}' |sed 's/\x1b\[[0-9;]*m//g')
		COMPREPLY=($(compgen -W "$languages" -- $2))
		;;
	*)
		COMPREPLY=($(compgen -fd -W "-h --help --tutor -V --version -v -vv -vvv --health -g --grammar --vsplit --hsplit" -- $2))
		;;
	esac
} && complete -F _hx hx

