# overthrow-app

Playable macroquad frontend: a human (player A) versus a scripted bot,
or a bot-vs-bot match played out for you to watch.
One crate, three build targets
— native desktop, web (WebAssembly) and Android —
all linking `overthrow-engine` and `overthrow-bot` directly,
with no FFI boundary.

Controls are documented at the top of `src/main.rs`
and printed in the in-game HUD.

## Native (desktop)

    cargo run -p overthrow-app --release

## Web (WebAssembly)

macroquad targets `wasm32-unknown-unknown` and runs through a small JS loader.
The loader (`mq_js_bundle.js`) and the page shell (`index.html`)
are committed in this directory;
you only need to build the `.wasm` and serve the three files together.

    rustup target add wasm32-unknown-unknown
    cargo build -p overthrow-app --release --target wasm32-unknown-unknown
    cp ../target/wasm32-unknown-unknown/release/overthrow-app.wasm .
    python3 -m http.server 8080   # then open http://localhost:8080

Serve over HTTP, not `file://` —
browsers refuse to fetch `.wasm` from the filesystem.
`mq_js_bundle.js` is the upstream miniquad loader
(not-fl3.github.io/miniquad-samples);
re-vendor it from there to update.

The steps above are for local iteration;
pushes to `main` build this target and publish it to GitHub Pages
through [`.github/workflows/pages.yml`](../.github/workflows/pages.yml).

## Android

Packaged with [`cargo-apk`](https://github.com/rust-mobile/cargo-apk),
which wraps the same crate in an APK via the NDK
(macroquad's windowing layer, miniquad, has a native Android backend,
so no code changes are needed).

    cargo install cargo-apk
    rustup target add aarch64-linux-android
    cargo apk run -p overthrow-app          # build, install and launch on a device

Requires the Android SDK + NDK with `ANDROID_HOME`/`ANDROID_NDK_ROOT` set.
Touch maps onto mouse input;
keyboard-only actions (end turn, new game) will want on-screen buttons
before this is comfortable on a phone — see the repo `TODO.md`.
