[![Rust](https://github.com/ngutech21/cashu-rs/actions/workflows/rust.yml/badge.svg?branch=master)](https://github.com/ngutech21/cashu-rs/actions/workflows/rust.yml)
[![coverage](https://img.shields.io/codecov/c/github/ngutech21/cashu-rs)](https://app.codecov.io/gh/ngutech21/cashu-rs/)
[![Flutter](https://github.com/ngutech21/cashu-rs/actions/workflows/flutter.yml/badge.svg?branch=master)](https://github.com/ngutech21/cashu-rs/actions/workflows/flutter.yml)


⚠️ **Don't be reckless:** This project is in early development, it does however work with real sats! Always use amounts you don't mind loosing. 

# cashu-rs
cashu-rs is a Chaumian Ecash library, mint, cli-wallet and flutter desktop-app. 

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
Wallet Features:

- [x] connect to mint (load keys)
- [x] request minting tokens
- [x] minting tokens
- [x] sending tokens (get encoded token for chosen value)
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
- [] [NUT-09](https://github.com/cashubtc/nuts/blob/main/09.md)







## Crates
- [core](./core) The core of the cashu library. Contains all the logic for creating and verifying tokens.
- [mint](./mint) Cashu mint server. Handles minting, melting and token requests.
- [wallet](./wallet) Cashu cli-wallet and library
- [flutter_bridge](./flutter/native) Thin wrapper using [flutter-rust-bridge](https://github.com/fzyzcjy/flutter_rust_bridge) around the wallet library for use in flutter. 

## Usage
### Setup rust
```
git clone https://github.com/ngutech21/cashu-rs.git
cargo install just typos-cli sqlx-cli grcov
rustup component add llvm-tools-preview
cd cashu-rs
```


### Config
```bash
mv .env.example .env
# edit .env file
vim .env
```

### Run mint (cashu-server)
To run the mint you need to setup a lightning regtest environment like [Polar](https://lightningpolar.com) and a Lnbits instance. In Lnbits create a new wallet and copy the admin key into the .env file and set the url to your Lnbits instance. The mint uses RocksDB for storing used proofs and pending invoices. You can set the path to the database in the .env file.
```
just run-mint
```


### Run cli-wallet
#### Check Balance
```
just run-wallet balance
```

#### Mint tokens
This command will return a Lightning invoice that you need to pay to mint new ecash tokens.
```
just run-wallet mint 42
```

#### Send tokens
To send tokens to another user, enter. The tokens will get printed to STOUT. You can then send them to the recipient via any messaging app.
```
just run-wallet send 21
```

#### Receive tokens
To receive tokens you need to enter the token as first argument to the receive command. The tokens will get verified and the value will be added to your balance.
```
just run-wallet receive cashuAeyJ0...
```



### Setup flutter
If you want to use the flutter app you need to setup flutter and the rust bridge:
- [Flutter SDK](https://docs.flutter.dev/get-started/install)
- `flutter_rust_bridge_codegen` [cargo package](https://cjycode.com/flutter_rust_bridge/integrate/deps.html#build-time-dependencies)
- Appropriate [Rust targets](https://rust-lang.github.io/rustup/cross-compilation.html) for cross-compiling to your device

### Run flutter desktop app (macOS only at the moment)
```
just flutter-run
```

### Development
To run coverage, check for typos, generate the flutter rust bridge etc. use the just commands:
```
Available recipes:
    coverage
    default
    final-check
    flutter-gen
    flutter-run
    run-mint
    run-wallet *ARGS
    typos
    typos-fix-all
```



## License

cashu-rs is distributed under the terms of the MIT license.
See [LICENSE](LICENSE).






