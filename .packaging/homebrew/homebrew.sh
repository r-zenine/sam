#!/bin/sh
cd `git rev-parse --show-toplevel`
cd .packaging/homebrew
git clone git@github.com:r-zenine/homebrew-ssam.git
cd homebrew-ssam
git config --global user.email "r.zenine@gmail.com"
git config --global user.name "Ryad ZENINE"
cp ../ssam.rb Formula/ssam.rb
git add --all
git commit -a -m"bump version"
git push origin main