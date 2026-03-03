# Niobium — build automation
#
# `make`        → full release build (Rust + Flutter + bundle .so)
# `make test`   → all tests + lint
# `make dev`    → debug build (faster iteration)

FLUTTER  ?= flutter
BUNDLE   := app/build/linux/x64/release/bundle
DBG_BUNDLE := app/build/linux/x64/debug/bundle
SO_NAME  := librust_lib_niobium.so

.PHONY: all build rust flutter bundle test lint clean dev run codegen

# ── Primary targets ──────────────────────────────────────────────────

all: build

build: rust flutter bundle  ## Full release build

dev: rust-debug flutter-debug bundle-debug  ## Debug build (faster)

run: build  ## Build and run the app
	$(BUNDLE)/niobium_app

# ── Rust ─────────────────────────────────────────────────────────────

rust:  ## Build Rust workspace (release)
	cargo build --release

rust-debug:
	cargo build

# ── Flutter ──────────────────────────────────────────────────────────

flutter:  ## Build Flutter desktop app (release)
	cd app && $(FLUTTER) build linux

flutter-debug:
	cd app && $(FLUTTER) build linux --debug

# ── Bundle .so ───────────────────────────────────────────────────────
# flutter_rust_bridge doesn't auto-bundle the .so — copy it manually.

bundle: rust flutter
	cp target/release/$(SO_NAME) $(BUNDLE)/lib/

bundle-debug: rust-debug flutter-debug
	cp target/debug/$(SO_NAME) $(DBG_BUNDLE)/lib/

# ── FRB codegen ──────────────────────────────────────────────────────

codegen:  ## Regenerate flutter_rust_bridge bindings
	cd app && flutter_rust_bridge_codegen generate

# ── Test & lint ──────────────────────────────────────────────────────

test: lint  ## Run all tests + lint
	cargo test
	cd app && $(FLUTTER) test

lint:  ## Static analysis
	cargo clippy -- -D warnings
	cd app && $(FLUTTER) analyze lib/

# ── Clean ────────────────────────────────────────────────────────────

clean:  ## Remove all build artifacts
	cargo clean
	cd app && $(FLUTTER) clean
