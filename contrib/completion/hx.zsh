#compdef _hx hx
# Zsh completion script for Helix editor

_hx() {
	_arguments -C \
		"-h[Prints help information]" \
		"--help[Prints help information]" \
		"-v[Increase logging verbosity]" \
		"-vv[Increase logging verbosity]" \
		"-vvv[Increase logging verbosity]" \
		"-V[Prints version information]" \
		"--version[Prints version information]" \
		"--tutor[Loads the tutorial]" \
		"--health[Checks for errors in editor setup]:language:->health" \
		"-g[Fetches or builds tree-sitter grammars]:action:->grammar" \
		"--grammar[Fetches or builds tree-sitter grammars]:action:->grammar" \
		"--vsplit[Splits all given files vertically]" \
		"--hsplit[Splits all given files horizontally]" \
		"-c[Specifies a file to use for configuration]" \
		"--config[Specifies a file to use for configuration]" \
		"-w[Specify initial working directory]" \
		"--working-dir[Specify initial working directory]" \
		"--log[Specifies a file to use for logging]" \
		"*:file:_files"

	case "$state" in
	health)
		local languages=($(hx --health | tail -n '+11' | awk '{print $1}' | sed 's/\x1b\[[0-9;]*m//g;s/[✘✓]//g'))
		_values 'language' $languages
		;;
	grammar)
		_values 'action' fetch build
		;;
	esac
}
