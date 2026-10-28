MAKEFLAGS += --no-print-directory

.DEFAULT_GOAL := help

.PHONY: build clean fmt help lint quality run test

help: ## Show available targets
	@echo "pforth (Go) - Available targets"
	@echo ""
	@grep -hE '^[a-zA-Z_-]+:.*## ' $(MAKEFILE_LIST) \
		| sort \
		| awk 'BEGIN {FS = ":.*## "} {printf "  %-15s %s\n", $$1, $$2}'

build: ## Compile binary
	@.make/build.sh

run: ## Run binary or interactive REPL
	@.make/run.sh "$$@"

test: ## Run unit tests
	@.make/test.sh

fmt: ## Format source files
	@go fmt ./...

lint: ## Run go vet
	@go vet ./...

quality: ## Run all quality checks
	@.make/quality.sh

clean: ## Remove build artifacts
	@rm -rf bin/pforth
