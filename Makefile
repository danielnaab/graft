.PHONY: build
build:
	docker build -t graft:local .

.PHONY: test
test:
	cd examples/minimal-kb && ../../bin/graft rebuild

.PHONY: shell
shell:
	docker run --rm -it -v $(PWD):/work graft:local bash

.PHONY: help
help:
	@echo "Available targets:"
	@echo "  build  - Build graft:local Docker image"
	@echo "  test   - Test graft with minimal-kb example"
	@echo "  shell  - Open interactive shell in graft container"
