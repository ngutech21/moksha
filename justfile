platform := if os_family() == "unix" { "macos"} else {os_family()}

# list all tasks
default:
  @just --list


# run the cashu-mint
run-mint:
  cargo run -q --bin cashurs-mint

# run the cli-wallet with given args
run-wallet *ARGS:
  WALLET_DB_PATH=./data/wallet/wallet.db cargo run --bin cashurs-cli {{ARGS}} 


# check code for typos
[no-exit-message]
typos:
  #!/usr/bin/env bash
  >&2 echo '💡 Valid new words can be added to `typos.toml`'
  typos


# fix all typos
[no-exit-message]
typos-fix-all:
  #!/usr/bin/env bash
  >&2 echo '💡 Valid new words can be added to `typos.toml`'
  typos --write-changes

# format code, check typos and run tests
final-check:
  cargo fmt --all
  just typos
  cargo test

#run coverage
coverage:
  #!/usr/bin/env bash
  mkdir -p target/coverage
  CARGO_INCREMENTAL=0 RUSTFLAGS='-Cinstrument-coverage' LLVM_PROFILE_FILE='cargo-test-%p-%m.profraw' cargo test
  grcov . --binary-path ./target/debug/deps/ -s . -t lcov --branch --ignore-not-existing --ignore '../*' --ignore "/*" -o target/coverage/tests.lcov
  grcov . --binary-path ./target/debug/deps/ -s . -t html --branch --ignore-not-existing --ignore '../*' --ignore "/*" -o target/coverage/html
  find . -name '*.profraw' -exec rm -r {} \;
  >&2 echo '💡 Created the report in target/coverage/html`'
  


# generate flutter-rust bridge
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

# run flutter desktop-app 
flutter-run:
    cd flutter && \
    flutter run -d {{ platform }}


# build flutter desktop-app
flutter-build:
    cd flutter && \
    flutter clean && \
    flutter build {{ platform }}
