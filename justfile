default:
  @just --list


run-mint:
  cargo run --bin cashurs-mint

run-wallet *ARGS:
  cargo run --bin cashurs-cli {{ARGS}} 


[no-exit-message]
typos:
  #!/usr/bin/env bash
  >&2 echo '💡 Valid new words can be added to `typos.toml`'
  typos

[no-exit-message]
typos-fix-all:
  #!/usr/bin/env bash
  >&2 echo '💡 Valid new words can be added to `typos.toml`'
  typos --write-changes

final-check:
  cargo fmt --all
  just typos
  cargo test


flutter-gen:
    cd flutter && \
    flutter pub get && \
    flutter_rust_bridge_codegen \
        --rust-input native/src/api.rs \
        --dart-output lib/generated/bridge_generated.dart \
        --c-output ios/Runner/bridge_generated.h \
        --extra-c-output-path macos/Runner/ \
        --dart-decl-output lib/generated/bridge_definitions.dart \
        --wasm

flutter-run:
    cd flutter && \
    flutter run -d macos
