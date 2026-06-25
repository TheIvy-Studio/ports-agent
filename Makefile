VERSION ?= 0.1.0
PKG_ARCH ?= amd64
DIST ?= dist

export VERSION
export PKG_ARCH

.PHONY: build check clean deb rpm apk arch packages

build:
	cargo build --release --locked

check:
	cargo check --workspace
	cargo clippy --workspace --all-targets

$(DIST):
	mkdir -p $(DIST)

deb: build $(DIST)
	nfpm package -f packaging/nfpm.yaml -p deb -t $(DIST)/

rpm: build $(DIST)
	nfpm package -f packaging/nfpm.yaml -p rpm -t $(DIST)/

apk: build $(DIST)
	nfpm package -f packaging/nfpm.yaml -p apk -t $(DIST)/

arch: build
	cd packaging/pacman && makepkg -f

packages: deb rpm apk

clean:
	cargo clean
	rm -rf $(DIST)
