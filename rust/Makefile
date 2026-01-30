MAKEFLAGS += --no-print-directory

.DEFAULT_GOAL := help

.PHONY: build clean help quality run test

help: ## Show available targets
	@$(MAKE) -C pforth help

build: ## Compile binary
	@$(MAKE) -C pforth build

run: ## Run binary
	@$(MAKE) -C pforth run

test: ## Run tests
	@$(MAKE) -C pforth test

quality: ## Format check and clippy
	@$(MAKE) -C pforth quality

clean: ## Remove build artifacts
	@$(MAKE) -C pforth clean
