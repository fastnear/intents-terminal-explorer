# NEARx Build Automation
# Option A: Static site with wasm-bindgen (no Trunk)

# Load environment variables from .env file if it exists
ifneq (,$(wildcard ./.env))
    include .env
    export
endif

.PHONY: help web web-release dev clean install-deps

help:
	@echo "NEARx Build Commands:"
	@echo "  make web          - Build web frontend (debug mode)"
	@echo "  make web-release  - Build web frontend (release mode, optimized)"
	@echo "  make dev          - Start local dev server for web/"
	@echo "  make clean        - Clean build artifacts"
	@echo "  make install-deps - Install required build tools"

# Build web frontend (debug mode)
web:
	@echo "🔨 Building WASM (debug)..."
	@FASTNEAR_AUTH_TOKEN="$(FASTNEAR_AUTH_TOKEN)" cargo build \
		--target wasm32-unknown-unknown \
		--no-default-features \
		--features dom-web \
		--bin nearx-web-dom
	@echo "🔗 Generating JS bindings..."
	@wasm-bindgen \
		--target web \
		--out-dir web/pkg \
		--out-name nearx_web_dom \
		--no-typescript \
		target/wasm32-unknown-unknown/debug/nearx-web-dom.wasm
	@echo "✅ Web build complete → web/"

# Build web frontend (release mode, optimized)
web-release:
	@echo "🔨 Building WASM (release, optimized)..."
	@FASTNEAR_AUTH_TOKEN="$(FASTNEAR_AUTH_TOKEN)" cargo build \
		--target wasm32-unknown-unknown \
		--no-default-features \
		--features dom-web \
		--bin nearx-web-dom \
		--release
	@echo "🔗 Generating JS bindings..."
	@wasm-bindgen \
		--target web \
		--out-dir web/pkg \
		--out-name nearx_web_dom \
		--no-typescript \
		target/wasm32-unknown-unknown/release/nearx-web-dom.wasm
	@echo "✅ Web build complete (release) → web/"

# Start local dev server
dev: web
	@if lsof -i :8000 > /dev/null 2>&1; then \
		echo "❌ Port 8000 is already in use. Kill the process or use 'make kill-dev'"; \
		exit 1; \
	else \
		echo "🚀 Starting dev server at http://localhost:8000"; \
		echo "   Press Ctrl+C to stop"; \
		cd web && python3 -m http.server 8000; \
	fi

# Kill any process using port 8000
kill-dev:
	@echo "🔫 Killing process on port 8000..."
	@lsof -ti :8000 | xargs kill -9 2>/dev/null || echo "No process found on port 8000"

# Clean build artifacts
clean:
	@echo "🧹 Cleaning build artifacts..."
	@cargo clean
	@rm -rf web/pkg
	@rm -rf dist
	@echo "✅ Clean complete"

# Install required build tools
install-deps:
	@echo "📦 Installing build dependencies..."
	@rustup target add wasm32-unknown-unknown
	@cargo install wasm-bindgen-cli --locked
	@echo "✅ Dependencies installed"
