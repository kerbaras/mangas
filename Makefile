.PHONY: build test coverage clean run

# Build the binary
build:
	go build -o bin/mangas ./cmd

# Run all tests
test:
	go test -v ./...

# Run tests with coverage
coverage:
	@echo "Running tests with coverage..."
	@go test -v -coverprofile=coverage.out ./pkg/...
	@echo "\n=== Core Business Logic Coverage ==="
	@go tool cover -func=coverage.out | grep -E "pkg/(data|integrations|services|sources)" | awk '{sum+=$$NF; count++} END {printf "Coverage: %.1f%%\n", sum/count}'
	@echo "\n=== Overall Coverage ==="
	@go tool cover -func=coverage.out | tail -1
	@echo "\n=== Detailed Package Coverage ==="
	@go test -coverprofile=coverage.out ./pkg/... 2>&1 | grep -E "coverage:|ok" | grep -v "no test files"

# Generate HTML coverage report
coverage-html: coverage
	go tool cover -html=coverage.out -o coverage.html
	@echo "Coverage report generated: coverage.html"

# Clean build artifacts
clean:
	rm -f bin/mangas
	rm -f coverage.out coverage.html
	rm -rf ~/.mangas/test*

# Run the application
run: build
	./bin/mangas

# Install dependencies
deps:
	go mod download
	go mod tidy

# Run linter
lint:
	@which golangci-lint > /dev/null || (echo "golangci-lint not installed" && exit 1)
	golangci-lint run

.DEFAULT_GOAL := build

