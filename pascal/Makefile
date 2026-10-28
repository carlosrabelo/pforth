MAKEFLAGS += --no-print-directory

.DEFAULT_GOAL := help

.PHONY: build clean help quality run test

help: ## Show available targets
	@echo "pforth (Free Pascal) - Available targets"
	@echo ""
	@grep -hE '^[a-zA-Z_-]+:.*## ' $(MAKEFILE_LIST) \
		| sort \
		| awk 'BEGIN {FS = ":.*## "} {printf "  %-15s %s\n", $$1, $$2}'

build: ## Compile binary
	@.make/build.sh

run: ## Run binary
	@.make/run.sh

test: ## Compile and run tests
	@.make/test.sh

quality: ## All quality checks
	@.make/quality.sh

clean: ## Remove build artifacts
	@rm -rf bin/pforth bin/test_runner obj/
