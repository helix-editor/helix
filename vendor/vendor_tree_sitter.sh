#!/usr/bin/env bash

REMOTE=https://github.com/tree-sitter/tree-sitter.git
BRANCH=v0.22.5

rm -rf tree-sitter
rm -rf tmp
git clone --depth 1 --branch $BRANCH $REMOTE tmp
mkdir tree-sitter
mv tmp/lib/src tree-sitter
mv tmp/lib/include tree-sitter
mv tmp/LICENSE tree-sitter
rm -rf tmp
