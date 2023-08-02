platform := if os_family() == "unix" { "macos"} else {os_family()}

# list all tasks
default:
  @just --list

# clean cargo and flutter
clean:
  cargo clean
  cd flutter && flutter clean

# run the cashu-mint
run-mint:
  RUST_BACKTRACE=1 cargo run --bin moksha-mint

# run the cli-wallet with given args
run-cli *ARGS:
  RUST_BACKTRACE=1 cargo run --bin moksha-cli -- -m http://127.0.0.1:3338 -d ./data/wallet  {{ARGS}} 


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
  just build-wasm

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
gen-flutter-bridge:
    cd flutter && \
    flutter pub get && \
    flutter_rust_bridge_codegen \
        --rust-input native/src/api.rs \
        --dart-output lib/generated/bridge_generated.dart \
        --c-output ios/Runner/bridge_generated.h \
        --extra-c-output-path macos/Runner/ \
        --dart-decl-output lib/generated/bridge_definitions.dart \
        --dart-format-line-length 120 \
        --dart-enums-style \
        --no-use-bridge-in-method \
        --wasm

# run flutter desktop-app 
run-flutter:
    cd flutter && \
    flutter run -d {{ platform }}


# build flutter desktop-app
build-flutter:
    cd flutter && \
    flutter clean && \
    flutter build {{ platform }}

# build the mint docker-image
build-docker:
    docker build -t moksha:latest .


# compile all rust crates, that are relevant for the client, to wasm
build-wasm:
   cargo build -p native  -p  moksha-core -p moksha-wallet -p moksha-fedimint --target wasm32-unknown-unknown
   
    
