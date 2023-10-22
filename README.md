[![crate](https://img.shields.io/crates/v/moksha-core.svg?logo=rust)](https://crates.io/crates/moksha-core)
[![rust](https://github.com/ngutech21/moksha/actions/workflows/rust.yml/badge.svg?branch=master)](https://github.com/ngutech21/moksha/actions/workflows/rust.yml)
[![wasm](https://github.com/ngutech21/moksha/actions/workflows/wasm.yml/badge.svg?branch=master)](https://github.com/ngutech21/moksha/actions/workflows/wasm.yml)
[![coverage](https://img.shields.io/codecov/c/github/ngutech21/moksha)](https://app.codecov.io/gh/ngutech21/moksha/)
[![flutter](https://github.com/ngutech21/moksha/actions/workflows/flutter.yml/badge.svg?branch=master)](https://github.com/ngutech21/moksha/actions/workflows/flutter.yml)

⚠️ **Don't be reckless:** This project is in early development, it does however work with real sats! Always use amounts you don't mind loosing.

# moksha

moksha is a cashu mint, cli-wallet and flutter desktop-app.

## Contents

- [About](#about)
- [Progress](#progress)
- [Crates](#crates)
- [Usage](#usage)
- [License](#license)

## About

Cashu is an Ecash implementation based on David Wagner's variant of Chaumian blinding. Token logic based
on [minicash](https://github.com/phyro/minicash) ([description](https://gist.github.com/phyro/935badc682057f418842c72961cf096c))
which implements a [Blind Diffie-Hellman Key Exchange](https://cypherpunks.venona.com/date/1996/03/msg01848.html) scheme
written down by Ruben Somsen [here](https://gist.github.com/RubenSomsen/be7a4760dd4596d06963d67baf140406).
Please read the [Cashu](https://github.com/callebtc/cashu) documentation for more detailed information.

## Progress

Mint Features:

- Supported backends
  - [x] LNbits
  - [x] Lnd
  - [x] Alby
  - [x] Strike  
  - [] core-lightning

Wallet Features:

- [x] connect to mint (load keys)
- [x] request minting tokens
- [x] minting tokens
- [x] sending tokens (get an encoded token for chosen value)
- [x] receiving tokens
- [x] melting tokens
- [] check if tokens are spent
- [] multi mint support

Implemented [NUTs](https://github.com/cashubtc/nuts/):

- [x] [NUT-00](https://github.com/cashubtc/nuts/blob/main/00.md)
- [x] [NUT-01](https://github.com/cashubtc/nuts/blob/main/01.md)
- [x] [NUT-02](https://github.com/cashubtc/nuts/blob/main/02.md)
- [x] [NUT-03](https://github.com/cashubtc/nuts/blob/main/03.md)
- [x] [NUT-04](https://github.com/cashubtc/nuts/blob/main/04.md)
- [x] [NUT-05](https://github.com/cashubtc/nuts/blob/main/05.md)
- [x] [NUT-06](https://github.com/cashubtc/nuts/blob/main/06.md)
- [] [NUT-07](https://github.com/cashubtc/nuts/blob/main/07.md)
- [] [NUT-08](https://github.com/cashubtc/nuts/blob/main/08.md)
- [x] [NUT-09](https://github.com/cashubtc/nuts/blob/main/09.md)

## Crates

- [moksha-core](./moksha-core) The core of the cashu library. Contains all the logic for creating and verifying tokens.
- [moksha-wallet](./moksha-wallet) Cashu wallet library
- [moksha-cli](./moksha-wallet) Cashu cli wallet
- [moksha-mint](./moksha-mint) Cashu mint server. Handles minting, melting and token requests.
- [moksha-fedimint](./moksha-fedimint) Fedimint wrapper for the cashu wallet
- [integrationtests](./integrationtests) Spins up a mint and runs integration tests against it.
- [flutter](./flutter/native) Thin wrapper using [flutter-rust-bridge](https://github.com/fzyzcjy/flutter_rust_bridge) around the wallet library for use in flutter.

## Usage

### Setup rust

```bash
git clone https://github.com/ngutech21/moksha.git
cargo install just typos-cli sqlx-cli grcov flutter_rust_bridge_codegen wasm-pack wasm-opt
rustup component add llvm-tools-preview
cd moksha
```

### Config

```bash
mv .env.example .env
# edit .env file
vim .env
```

### Run mint (cashu-server)

To run the mint you need to setup a lightning regtest environment like [Polar](https://lightningpolar.com) and a Lnbits or Lnd instance. In Lnbits create a new wallet and copy the admin key into the .env file and set the url to your Lnbits instance. The mint uses RocksDB for storing used proofs and pending invoices. You can set the path to the database in the .env file.

```bash
just run-mint
```

### Run cli-wallet

#### Show info

Shows the current version, database-dir and mint-url.

```bash
just run-cli info
```

#### Check Balance

```bash
just run-cli balance
```

#### Mint tokens

This command will return a Lightning invoice that you need to pay to mint new ecash tokens.

```bash
just run-cli mint 42
```

#### Send tokens

To send tokens to another user, enter. The tokens will get printed to STOUT. You can then send them to the recipient via any messaging app.

```bash
just run-cli send 21
```

#### Receive tokens

To receive tokens you need to enter the token as first argument to the receive command. The tokens will get verified and the value will be added to your balance.

```bash
just run-cli receive cashuAeyJ0...
```

### Setup flutter

If you want to use the flutter app you need to setup flutter and the rust bridge:

- [Flutter SDK](https://docs.flutter.dev/get-started/install)
- `flutter_rust_bridge_codegen` [cargo package](https://cjycode.com/flutter_rust_bridge/integrate/deps.html#build-time-dependencies)
- Appropriate [Rust targets](https://rust-lang.github.io/rustup/cross-compilation.html) for cross-compiling to your device

### update flutter dependencies

```bash
cd flutter
flutter pub get
```

### Run flutter desktop app

```bash
just run-desktop
```

### Development

To run coverage, check for typos, generate the flutter rust bridge etc. use the just commands:

```bash
Available recipes:
    build-desktop      # build flutter desktop-app
    build-docker       # build the mint docker-image
    build-wasm         # compile all rust crates, that are relevant for the client, to wasm
    build-web          # build flutter web-app in flutter/build/web
    clean              # clean cargo and flutter
    coverage           # run coverage
    default            # list all tasks
    final-check        # format code, check typos and run tests
    gen-flutter-bridge # generate flutter-rust bridge
    run-cli *ARGS      # run cli-wallet with the given args
    run-desktop        # run flutter desktop-app
    run-mint           # run the cashu-mint
    run-web            # run flutter web-app
    typos              # check code for typos
    typos-fix-all      # fix all typos
```

## License

moksha is distributed under the terms of the MIT license.
See [LICENSE](LICENSE).
