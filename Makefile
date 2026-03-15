GIT_COMMIT	?= $(shell git rev-parse HEAD)
GIT_TAG	?= $(shell git tag --points-at HEAD)
DIST_TYPE	?= snapshot
BRANCH	?= $(shell git rev-parse --abbrev-ref HEAD)
TARGET_PLATFORM = ${TARGET_PLATFORM}
PROJECT = sam
ARCH ?= $(shell uname -m | sed 's/arm64/aarch64/')

build:
	cargo build --release

build-mcp:
	cargo build --release -p sam-mcp

check: build
	cargo clippy && cargo check && cargo fmt -- --check

test: build check
	cargo test --release --workspace
	python3 agent-skills/hooks/test_block_sam_direct_access.py

unused_deps: 
	cargo +nightly udeps --all-targets

package_linux: test check build version
	cd ./target/release/ && tar -czvf $(PROJECT)_linux_$(ARCH)_$(VERSION).tar.gz $(PROJECT) $(PROJECT)-mcp

package_macos: test check build version
	cd ./target/release/ && tar -czvf $(PROJECT)_macos_$(ARCH)_$(VERSION).tar.gz $(PROJECT) $(PROJECT)-mcp

package_macos_x86: test check version
	cargo build --release --target x86_64-apple-darwin
	cd ./target/x86_64-apple-darwin/release/ && tar -czvf ../../../target/release/$(PROJECT)_macos_x86_64_$(VERSION).tar.gz $(PROJECT) $(PROJECT)-mcp

package_debian:
	cargo deb -p sam-cli

publish_version: 
	cargo workspaces version

version:
	$(info =====  $@  =====)
ifneq ($(GIT_TAG),)
	$(eval VERSION := $(GIT_TAG))
	$(eval VERSION_NUMBER := $(subst v,,$(VERSION)))
else
	$(eval VERSION := $(GIT_COMMIT)-SNAPSHOT)
endif
	$(info $(VERSION) on commit $(GIT_COMMIT))
	$(info $(VERSION_NUMBER) on commit $(GIT_COMMIT))

release_upload: package
ifneq ($(GIT_TAG),)
	gh release upload $(VERSION) ./$(PROJECT)_$(TARGET_PLATFORM)_$(VERSION).tar.gz 
endif

create_release: version
ifneq ($(GIT_TAG),)
	gh release create -t "Release $(VERSION)" -n "" --target master $(GIT_TAG)
	gh release upload $(GIT_TAG) ./target/release/$(PROJECT)_linux_x86_64_$(VERSION).tar.gz 
	gh release upload $(GIT_TAG) ./target/release/$(PROJECT)_macos_x86_64_$(VERSION).tar.gz 
endif

.PHONY: version create_release publish_version build-mcp
