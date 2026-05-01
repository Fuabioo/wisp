default: build

# Build a minimally sized optimized Go binary
build:
	go build -trimpath -ldflags="-s -w" -o wisp .

# Ultra-compressed build (requires 'upx' to be installed on your system)
build-micro: build
	upx --best --lzma wisp

# Run the wisp daemon
daemon:
	go run . daemon

# Run the project
run:
	go run .

# Format code
fmt:
	go fmt ./...

# Tidy dependencies
tidy:
	go mod tidy

# Install tools to analyze and reduce binary size
install-deps:
	go install github.com/loov/goda@latest
	GOEXPERIMENT=jsonv2 go install github.com/Zxilly/go-size-analyzer/cmd/gsa@latest
	@if command -v brew >/dev/null 2>&1; then brew install upx; \
	elif command -v apt-get >/dev/null 2>&1; then sudo apt-get install -y upx-ucl; \
	else echo "Please install UPX manually for your system."; fi

# Install an optimized dev build to ~/.local/bin to avoid colliding with a prod install
install-dev: build-micro
	mkdir -p ~/.local/bin
	mv wisp ~/.local/bin/wisp-dev
	@echo "wisp-dev installed to ~/.local/bin/wisp-dev"

# Check if required tools are installed
check-deps:
	@command -v goda >/dev/null 2>&1 && echo "✅ goda is installed" || echo "❌ goda is missing (run: just install-deps)"
	@command -v gsa >/dev/null 2>&1 && echo "✅ gsa is installed" || echo "❌ gsa is missing (run: just install-deps)"
	@command -v upx >/dev/null 2>&1 && echo "✅ upx is installed" || echo "❌ upx is missing (run: just install-deps)"
