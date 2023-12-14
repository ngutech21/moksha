export CFLAGS := ""


# list all tasks
default:
  @just --list

# install all dependencies
deps:
  cargo install flutter_rust_bridge_codegen@1.82.5 sqlx-cli typos-cli  grcov wasm-pack wasm-opt just


# clean cargo and flutter
clean:
  cargo clean
  cd flutter && flutter clean


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

# run the cashu-mint
run-mint:
  RUST_BACKTRACE=1 MINT_APP_ENV=dev cargo run --bin moksha-mint

# run cli-wallet with the given args
run-cli *ARGS:
  RUST_BACKTRACE=1 cargo run --bin moksha-cli -- -m http://127.0.0.1:3338 -d ./data/wallet  {{ARGS}} 


# run flutter desktop-app 
run-desktop:
    cd flutter && \
    flutter run -d {{ os() }}

# run flutter web-app
run-web:
    cd flutter && \
    dart run flutter_rust_bridge:serve 


# build flutter desktop-app
build-desktop:
    cd flutter && \
    flutter clean && \
    flutter build {{ os() }}


# build the mint docker-image
build-docker:
    docker build -t moksha:latest .


# build flutter web-app in flutter/build/web
build-web: build-wasm
  cd flutter && \
  flutter clean && \
  RUSTFLAGS="-C target-feature=+atomics,+bulk-memory,+mutable-globals" RUSTUP_TOOLCHAIN=nightly wasm-pack build -t no-modules -d  $(pwd)/web/pkg --no-typescript --out-name native --dev native -- -Z build-std=std,panic_abort && \
  wasm-opt -Oz -o $(pwd)/web/pkg/native_bg.wasm $(pwd)/web/pkg/native_bg.wasm && \
  flutter build web --profile
  


# compile all rust crates, that are relevant for the client, to wasm
build-wasm:
   RUSTFLAGS="-C target-feature=+atomics,+bulk-memory,+mutable-globals" cargo +nightly build -p native -p  moksha-core -p moksha-wallet -p moksha-fedimint \
   --target wasm32-unknown-unknown \
   -Z build-std=std,panic_abort


db-prepare:
  cd moksha-mint && \
  cargo sqlx prepare --database-url postgres://postgres:postgres@127.0.0.1/moksha-mint

db-create:
  cd moksha-mint && \
  cargo sqlx database create --database-url postgres://postgres:postgres@localhost/moksha-mint


fly-db-proxy:
  flyctl proxy 6542:5432 -a moksha-mint-db

db-secrets:
  flyctl secrets set LND_MACAROON="$(cat data/mutinynet/admin.macaroon)"
  flyctl secrets set LND_TLS_CERT="$(cat data/mutinynet/tls.cert)"
   
