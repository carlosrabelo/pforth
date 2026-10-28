MAKEFLAGS += --no-print-directory

.DEFAULT_GOAL := help

.PHONY: build clean help quality test

help: ## Show available targets
	@echo "pforth - Available targets"
	@echo ""
	@grep -hE '^[a-zA-Z_-]+:.*## ' $(MAKEFILE_LIST) \
		| sort \
		| awk 'BEGIN {FS = ":.*## "} {printf "  %-15s %s\n", $$1, $$2}'

build: ## Build all components
	@$(MAKE) -C cpp build

test: ## Run tests across all components
	@$(MAKE) -C cpp test

quality: ## Run quality checks across all components
	@$(MAKE) -C cpp quality

clean: ## Remove build artifacts
	@$(MAKE) -C cpp clean
