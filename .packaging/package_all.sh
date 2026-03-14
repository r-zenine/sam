#!/bin/sh
# change directory to top root of git project.
cd `git rev-parse --show-toplevel`
export PROJECT="sam"
export PROJECT_URL="https://github.com/r-zenine/sam"
export PROJECT_LICENCE="GPL-3.0"
export PROJECT_DESCRIPTION="sam lets you define custom aliases and search them using fuzzy search."
export APP_VERSION=$(grep sam-cli/Cargo.toml -e "^version = " |sed 's/version \=//' |sed 's/\"//g'|sed 's/ //g')
export ARCHIVE_X86_64="./target/release/sam_macos_x86_64_v${APP_VERSION}.tar.gz"
export ARCHIVE_AARCH64="./target/release/sam_macos_aarch64_v${APP_VERSION}.tar.gz"
export RELEASE_HASH_X86_64=$(sha256sum ${ARCHIVE_X86_64} | cut -d\  -f1)
export RELEASE_HASH_AARCH64=$(sha256sum ${ARCHIVE_AARCH64} | cut -d\  -f1)

envsubst < .packaging/homebrew/sam.rb.j2 > .packaging/homebrew/sam.rb
# envsubst < .packaging/snap/snapcraft.yaml.j2 > .packaging/snap/snapcraft.yaml

./.packaging/homebrew/homebrew.sh ## && ./.packaging/snap/snap.sh
