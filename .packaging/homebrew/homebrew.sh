#!/bin/sh
cd `git rev-parse --show-toplevel`
git submodule update --init --recursive
cd .packaging/homebrew
cp ssam.rb homebrew-ssam/Formula/ssam.rb
cd homebrew-ssam
git add --all
git commit -a -m"bump version"