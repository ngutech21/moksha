default:
  @just --list


run-mint:
  cargo run --bin cashurs-mint

run-wallet *ARGS:
  cargo run --bin cashurs-wallet {{ARGS}} 


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
