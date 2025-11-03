.PHONY: build
build:
	docker build -t docflow:local .

.PHONY: test
test:
	cd examples/minimal-kb && ../../bin/docflow rebuild

.PHONY: shell
shell:
	docker run --rm -it -v $(PWD):/work docflow:local bash

.PHONY: help
help:
	@echo "Available targets:"
	@echo "  build  - Build docflow:local Docker image"
	@echo "  test   - Test docflow with minimal-kb example"
	@echo "  shell  - Open interactive shell in docflow container"
