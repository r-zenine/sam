#!/bin/sh
cd `git rev-parse --show-toplevel`
cd .packaging/homebrew
git clone git@github.com:r-zenine/homebrew-sam.git
cd homebrew-sam
git config --global user.email "r.zenine@gmail.com"
git config --global user.name "Ryad ZENINE"
cp ../sam.rb Formula/sam.rb
git add --all
git commit -a -m"bump version"
git push origin main