MAKEFLAGS += --no-print-directory

.DEFAULT_GOAL := help

.PHONY: build clean help quality run setup test

help: ## Show available targets
	@grep -hE '^[a-zA-Z_-]+:.*## ' $(MAKEFILE_LIST) \
		| sort \
		| awk 'BEGIN {FS = ":.*## "} {printf "  %-15s %s\n", $$1, $$2}'

setup: ## Download dependencies for cpp
	@$(MAKE) -C cpp setup

build: ## Compile cpp
	@$(MAKE) -C cpp build

run: ## Run cpp binary
	@$(MAKE) -C cpp run

test: ## Test cpp
	@$(MAKE) -C cpp test

quality: ## Quality checks for cpp
	@$(MAKE) -C cpp quality

clean: ## Remove build artifacts
	@$(MAKE) -C cpp clean
