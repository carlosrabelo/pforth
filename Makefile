MAKEFLAGS += --no-print-directory

.DEFAULT_GOAL := help

.PHONY: build clean help quality run setup test

COMPONENTS := cpp go pascal rust

help: ## Show available targets
	@grep -hE '^[a-zA-Z_-]+:.*## ' $(MAKEFILE_LIST) \
		| sort \
		| awk 'BEGIN {FS = ":.*## "} {printf "  %-15s %s\n", $$1, $$2}'

setup: ## Download dependencies
	@for c in $(COMPONENTS); do $(MAKE) -C $$c setup 2>/dev/null || true; done

build: ## Compile components
	@for c in $(COMPONENTS); do $(MAKE) -C $$c build; done

run: ## Run cpp binary by default
	@$(MAKE) -C cpp run

test: ## Test components
	@for c in $(COMPONENTS); do $(MAKE) -C $$c test; done

quality: ## Quality checks
	@for c in $(COMPONENTS); do $(MAKE) -C $$c quality; done

clean: ## Remove build artifacts
	@for c in $(COMPONENTS); do $(MAKE) -C $$c clean; done
