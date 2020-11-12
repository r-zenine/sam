#!/bin/sh
git submodule --init
git submodule --update --recursive
cd `git rev-parse --show-toplevel`
cd .packaging/homebrew
cp ssam.rb homebrew-ssam/Formula/ssam.rb
cd homebrew-ssam
git add --all
git commit -a -m"bump version"