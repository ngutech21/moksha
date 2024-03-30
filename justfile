DB_URL := "postgres://postgres:postgres@localhost/moksha-mint"

# list all tasks
default:
  @just --list

# install all dependencies
deps:
  cargo install sqlx-cli typos-cli  grcov wasm-pack wasm-opt just


# clean cargo
clean:
  cargo clean


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
  just run-itests
  just build-wasm


# run coverage and create a report in html and lcov format
run-coverage:
  just run-coverage-tests
  just run-coverage-report

# runs all tests with coverage instrumentation
run-coverage-tests:
  docker compose --profile itest up -d
  RUST_BACKTRACE=1 CARGO_INCREMENTAL=0 RUSTFLAGS='-Cinstrument-coverage' LLVM_PROFILE_FILE='cargo-test-%p-%m.profraw' cargo test -- --test-threads=1
  docker compose --profile itest down

# creates a coverage report in html and lcov format
run-coverage-report:
  #!/usr/bin/env bash
  mkdir -p target/coverage
  grcov . --binary-path ./target/debug/ -s . -t lcov,html --branch --ignore-not-existing --ignore "*cargo*" --ignore "./data/*" --ignore "*/examples/*" -o target/coverage/
  find . -name '*.profraw' -exec rm -r {} \;
  >&2 echo '💡 Created the report in html-format target/coverage/html/index.html'


# run the cashu-mint
run-mint *ARGS:
  RUST_BACKTRACE=1 MINT_APP_ENV=dev cargo run --bin moksha-mint -- {{ARGS}}

# run cli-wallet with the given args
run-cli *ARGS:
  RUST_BACKTRACE=1 cargo run --bin moksha-cli -- -m http://127.0.0.1:3338 -d ./data/wallet  {{ARGS}} 

# runs all tests
run-tests:
  RUST_BACKTRACE=1 cargo test --workspace --exclude integrationtests


# checks if docker and docker compose is installed and running
_check-docker:
  #!/usr/bin/env bash
  if ! command -v docker &> /dev/null; then
    >&2 echo 'Error: Docker is not installed.';
    exit 1;
  fi

  if ! command -v docker compose &> /dev/null; then
   >&2 echo 'Error: Docker Compose is not installed.' >&2;
   exit 1;
  fi

  if ! command docker info &> /dev/null; then
    >&2 echo 'Error: Docker is not running.';
    exit 1;
  fi

# starts bitcoind, nutshell, 2 lnd nodes via docker and runs the integration tests
run-itests: _check-docker
    docker compose --profile itest up -d
    RUST_BACKTRACE=1 cargo test -p integrationtests -- --test-threads=1
    docker compose --profile itest down

# build the mint docker-image
build-docker:
    docker build --file Dockerfile.alpine --build-arg COMMITHASH=$(git rev-parse HEAD) --build-arg BUILDTIME=$(date -u '+%F-%T') -t moksha-mint:latest .


# compile all rust crates, that are relevant for the client, to wasm
build-wasm:
   cargo +nightly build -p  moksha-core -p moksha-wallet \
   --target wasm32-unknown-unknown \
   -Z build-std=std,panic_abort


# runs sqlx prepare
db-prepare:
  cd moksha-mint && \
  cargo sqlx prepare --database-url {{ DB_URL }}

# runs sqlx prepare
db-migrate:
  cd moksha-mint && \
  cargo sqlx migrate run --database-url {{ DB_URL }}

# creates the postgres database
db-create:
  cd moksha-mint && \
  cargo sqlx database create --database-url {{ DB_URL }}


# publish everything on crates.io
publish:
  cargo publish -p moksha-core
  cargo publish -p moksha-wallet
  cargo publish -p moksha-mint
  cargo publish -p moksha-cli

   
