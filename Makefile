.PHONY: all build test clippy clippy-fix fmt clean doc release check

# Default target
all: fmt clippy test

# Build the project
build:
	cargo build

# Run tests
test:
	cargo test --all-features

# Run clippy with the same settings as CI
clippy:
	cargo clippy -- \
		-D clippy::all \
		-D clippy::pedantic \
		-D clippy::nursery \
		-D clippy::cargo \
		-A clippy::module_name_repetitions \
		-A clippy::must_use_candidate \
		-A clippy::missing_errors_doc \
		-A clippy::missing_panics_doc \
		-A clippy::missing_docs_in_private_items \
		-A clippy::missing_const_for_fn
	cargo clippy --tests -- \
		-D clippy::all \
		-D clippy::pedantic \
		-D clippy::nursery \
		-D clippy::cargo \
		-A clippy::module_name_repetitions \
		-A clippy::must_use_candidate \
		-A clippy::missing_errors_doc \
		-A clippy::missing_panics_doc \
		-A clippy::missing_docs_in_private_items \
		-A clippy::missing_const_for_fn
	cargo clippy --examples -- \
		-D clippy::all \
		-D clippy::pedantic \
		-D clippy::nursery \
		-D clippy::cargo \
		-A clippy::module_name_repetitions \
		-A clippy::must_use_candidate \
		-A clippy::missing_errors_doc \
		-A clippy::missing_panics_doc \
		-A clippy::missing_docs_in_private_items \
		-A clippy::missing_const_for_fn \
		-A clippy::uninlined_format_args \
		-A clippy::map_unwrap_or \
		-A clippy::manual_let_else \
		-A clippy::needless_collect \
		-A clippy::single_match_else \
		-A clippy::option_if_let_else

# Run clippy and fix what can be fixed automatically
clippy-fix:
	cargo clippy --fix --allow-staged --allow-dirty

# Format code
fmt:
	cargo fmt

# Clean build artifacts
clean:
	cargo clean

# Build documentation
doc:
	cargo doc --no-deps --open

# Run all checks (what CI does)
check: fmt clippy test doc
	@echo "All checks passed!"

