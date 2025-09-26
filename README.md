# Keypress Racer: 1000 (Rust → WebAssembly)

GitHub Pages–ready build. Press any key 1000 times; counts one per full press→release.

## Local dev
```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
make build
python3 -m http.server 8000
# open http://localhost:8000/docs/
```

## Deploy (GitHub Pages via Actions)
- Push this repo to GitHub.
- Go to **Settings → Pages** and set **Source** to *GitHub Actions*.
- The included workflow `.github/workflows/pages.yml` builds to `docs/pkg` and deploys `docs/` automatically on push to `main`/`master`.
