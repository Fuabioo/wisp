default: build-micro

# Build a minimally sized optimized Go binary
build:
    go build -trimpath -ldflags="-s -w -X 'github.com/Fuabioo/wisp/cmd.Version=dev' -X 'github.com/Fuabioo/wisp/cmd.CommitSHA=$(git rev-parse --short HEAD 2>/dev/null || echo none)' -X 'github.com/Fuabioo/wisp/cmd.BuildDate=$(date -u +%Y-%m-%dT%H:%M:%SZ)'" -o wisp .

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

# Lint code (gofmt check + go vet)
lint:
    @unformatted=$(gofmt -l .); \
    if [ -n "$unformatted" ]; then \
        echo "❌ gofmt issues:"; echo "$unformatted"; exit 1; \
    fi
    go vet ./...

# Install the pre-commit hook (runs `just lint` before each commit)
install-hooks:
    git config core.hooksPath .githooks
    @echo "✅ pre-commit hook installed (.githooks/pre-commit)"

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
    @echo "wisp-dev installed to ~/.local/bin/wisp-dev" && wisp-dev --version

# Check if required tools are installed
check-deps:
    @command -v goda >/dev/null 2>&1 && echo "✅ goda is installed" || echo "❌ goda is missing (run: just install-deps)"
    @command -v gsa >/dev/null 2>&1 && echo "✅ gsa is installed" || echo "❌ gsa is missing (run: just install-deps)"
    @command -v upx >/dev/null 2>&1 && echo "✅ upx is installed" || echo "❌ upx is missing (run: just install-deps)"

# Provision wisp as a user-level systemd daemon
provision: build-micro
    mkdir -p ~/.local/bin
    cp wisp ~/.local/bin/wisp
    mkdir -p ~/.config/systemd/user
    @echo "[Unit]" > ~/.config/systemd/user/wisp.service
    @echo "Description=Wisp Terminal Daemon" >> ~/.config/systemd/user/wisp.service
    @echo "After=network.target" >> ~/.config/systemd/user/wisp.service
    @echo "" >> ~/.config/systemd/user/wisp.service
    @echo "[Service]" >> ~/.config/systemd/user/wisp.service
    @echo "Type=simple" >> ~/.config/systemd/user/wisp.service
    @echo "ExecStart=%h/.local/bin/wisp daemon" >> ~/.config/systemd/user/wisp.service
    @echo "Restart=on-failure" >> ~/.config/systemd/user/wisp.service
    @echo "" >> ~/.config/systemd/user/wisp.service
    @echo "[Install]" >> ~/.config/systemd/user/wisp.service
    @echo "WantedBy=default.target" >> ~/.config/systemd/user/wisp.service
    systemctl --user daemon-reload
    systemctl --user enable --now wisp.service
    @echo "✅ Wisp daemon provisioned and started via systemd!"
