MAKEFLAGS += --no-print-directory

.DEFAULT_GOAL := help

.PHONY: build clean help quality run setup test

help: ## Show available targets
	@$(MAKE) -C pforth help

setup: ## Download dependencies
	@$(MAKE) -C pforth setup

build: ## Compile binary
	@$(MAKE) -C pforth build

run: ## Run binary
	@$(MAKE) -C pforth run

test: ## Compile and run tests
	@$(MAKE) -C pforth test

quality: ## All quality checks
	@$(MAKE) -C pforth quality

clean: ## Remove build artifacts
	@$(MAKE) -C pforth clean
