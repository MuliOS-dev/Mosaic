# Mosaic (Tauri + Leptos)

Native desktop app: Rust/Leptos UI (WASM) inside a Tauri v2 window.

## One-time setup
    rustup target add wasm32-unknown-unknown
    cargo install trunk tauri-cli --locked
    # Linux only: sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev build-essential libssl-dev
    # Windows: Microsoft C++ Build Tools + WebView2 (preinstalled on Win 11)
    # macOS: Xcode command line tools

## Run (native window, hot reload)
    cargo tauri dev

## Build installer / executable
    cargo tauri build

## Icons
Regenerate every platform icon (incl. .icns for macOS) from app-icon.png:
    cargo tauri icon app-icon.png

Note: Tailwind and Google Fonts still load from their CDNs, so the first
launch needs internet. Bundle them locally for fully offline use.
