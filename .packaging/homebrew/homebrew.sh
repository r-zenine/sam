#!/bin/sh
cd `git rev-parse --show-toplevel`
git config --global user.email "r.zenine@gmail.com"
git config --global user.name "Ryad ZENINE"
git submodule update --init --recursive
cd .packaging/homebrew
cp ssam.rb homebrew-ssam/Formula/ssam.rb
cd homebrew-ssam
git add --all
git commit -a -m"bump version"