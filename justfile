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
