# Agenda (Tauri + Leptos)

Native desktop calendar. Rust/Leptos UI (WASM) in a Tauri v2 window.

## One-time setup
    rustup target add wasm32-unknown-unknown
    cargo install trunk tauri-cli --locked
    # Linux: sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev build-essential libssl-dev

## Run / build
    cargo tauri dev
    cargo tauri build

Icons come from app-icon.png (`cargo tauri icon app-icon.png` regenerates them).
Tailwind and Google Fonts load from CDNs, so first launch needs internet.
