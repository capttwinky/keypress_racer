# Keypress Race (GitHub Pages ready)
# Usage:
#   make build     # wasm -> docs/pkg
#   make serve     # local http server at project root (open /docs/)
#   make clean     # remove build artifacts
#   make rebuild   # clean + build

WASM_PACK ?= wasm-pack
PORT ?= 8000

.PHONY: build serve clean rebuild help

build:
	$(WASM_PACK) build --release --target web --out-dir docs/pkg

serve:
	python3 -m http.server $(PORT)
	@echo "Open: http://localhost:$(PORT)/docs/"

clean:
	rm -rf pkg target docs/pkg

rebuild: clean build

help:
	@echo "Targets:"
	@echo "  build    - Build WASM package to ./docs/pkg for GitHub Pages"
	@echo "  serve    - Serve project root; open /docs/"
	@echo "  clean    - Remove ./pkg, ./target, and ./docs/pkg"
	@echo "  rebuild  - Clean then build"
