GIT_COMMIT	?= $(shell git rev-parse HEAD)
GIT_TAG	?= $(shell git tag --points-at HEAD)
DIST_TYPE	?= snapshot
BRANCH	?= $(shell git rev-parse --abbrev-ref HEAD)
TARGET_PLATFORM = ${TARGET_PLATFORM}
PROJECT = sam

build: 
	cargo build --release

check: build
	cargo clippy && cargo check && cargo fmt -- --check

test: build check
	cargo test --release

package: test check build version 
	tar -czvf ./$(PROJECT)_$(TARGET_PLATFORM)_$(VERSION).tar.gz target/release/$(PROJECT)
    
version:
	$(info =====  $@  =====)
ifneq ($(GIT_TAG),)
	$(eval VERSION := $(GIT_TAG))
else
	$(eval VERSION := $(subst /,-,$(BRANCH)))
	$(eval VERSION_FILE := $(GIT_COMMIT)-SNAPSHOT)
endif
	@test -n "$(VERSION)"
	$(info $(VERSION)$(VERSION_FILE) on commit $(GIT_COMMIT))

.PHONY: version