MAKEFLAGS += --no-print-directory

.DEFAULT_GOAL := help

.PHONY: build clean help quality run test

help: ## Show available targets
	@echo "pforth (Rust) - Available targets"
	@echo ""
	@grep -hE '^[a-zA-Z_-]+:.*## ' $(MAKEFILE_LIST) \
		| sort \
		| awk 'BEGIN {FS = ":.*## "} {printf "  %-15s %s\n", $$1, $$2}'

build: ## Compile binary
	@../.make/build.sh

run: ## Run binary
	@../.make/run.sh

test: ## Run tests
	@../.make/test.sh

quality: ## Format check and clippy
	@../.make/quality.sh

clean: ## Remove build artifacts
	@../.make/clean.sh
