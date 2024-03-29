[![crate](https://img.shields.io/crates/v/moksha-core.svg?logo=rust)](https://crates.io/crates/moksha-core)
[![rust](https://github.com/ngutech21/moksha/actions/workflows/rust.yml/badge.svg?branch=master)](https://github.com/ngutech21/moksha/actions/workflows/rust.yml)
[![wasm](https://github.com/ngutech21/moksha/actions/workflows/wasm.yml/badge.svg?branch=master)](https://github.com/ngutech21/moksha/actions/workflows/wasm.yml)
[![coverage](https://img.shields.io/codecov/c/github/ngutech21/moksha)](https://app.codecov.io/gh/ngutech21/moksha/)

⚠️ **Don't be reckless:** This project is in early development, it does however work with real sats! Always use amounts you don't mind loosing.

# moksha

moksha is a cashu library, mint and cli-wallet written in Rust.

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
  - [ ] core-lightning (WIP)

Wallet Features:

- [x] connect to mint (load keys)
- [x] request minting tokens
- [x] minting tokens
- [x] sending tokens (get an encoded token for chosen value)
- [x] receiving tokens
- [x] melting tokens
- [ ] check if tokens are spent
- [ ] multi mint support

Implemented [NUTs](https://github.com/cashubtc/nuts/):

- [x] [NUT-00](https://github.com/cashubtc/nuts/blob/main/00.md)
- [x] [NUT-01](https://github.com/cashubtc/nuts/blob/main/01.md)
- [x] [NUT-02](https://github.com/cashubtc/nuts/blob/main/02.md)
- [x] [NUT-03](https://github.com/cashubtc/nuts/blob/main/03.md)
- [x] [NUT-04](https://github.com/cashubtc/nuts/blob/main/04.md)
- [x] [NUT-05](https://github.com/cashubtc/nuts/blob/main/05.md)
- [x] [NUT-06](https://github.com/cashubtc/nuts/blob/main/06.md)
- [ ] [NUT-07](https://github.com/cashubtc/nuts/blob/main/07.md)
- [x] [NUT-08](https://github.com/cashubtc/nuts/blob/main/08.md)
- [ ] [NUT-09](https://github.com/cashubtc/nuts/blob/main/09.md)
- [ ] [NUT-10](https://github.com/cashubtc/nuts/blob/main/10.md)
- [ ] [NUT-11](https://github.com/cashubtc/nuts/blob/main/11.md)
- [ ] [NUT-12](https://github.com/cashubtc/nuts/blob/main/12.md)
- [x] [NUT-13](https://github.com/cashubtc/nuts/blob/main/13.md)
- [ ] [NUT-14](https://github.com/cashubtc/nuts/blob/main/14.md)
- [ ] [NUT-15](https://github.com/cashubtc/nuts/blob/main/15.md)
- [ ] [NUT-16](https://github.com/cashubtc/nuts/blob/main/16.md)
- [x] NUT-17 on-chain mint (unofficial nut)
- [x] NUT-18 on-chain melt (unofficial nut)

## Crates

- [moksha-core](./moksha-core) The core of the cashu library. Contains all the logic for creating and verifying tokens.
- [moksha-wallet](./moksha-wallet) Cashu wallet library
- [moksha-cli](./moksha-wallet) Cashu cli wallet
- [moksha-mint](./moksha-mint) Cashu mint server. Handles minting, melting and token requests.
- [integrationtests](./integrationtests) Spins up a mint and runs integration tests against it.

## Usage

### Deploy mint

#### Docker-compose

docker-compose simplifies the process of running multi-container Docker applications. Here's how you can use it to run the moksha -mint:

1. First, you need to have Docker Compose installed on your machine. If it's not installed, you can download it from the [official Docker website](https://docs.docker.com/compose/install/).

2. Copy the tls.cert and admin.macaroon files from your LND instance into the `./data/mutinynet/` directory.

3. Configure the `LND_GRPC_HOST` environment variable in the `docker-compose.yml` file to point to your LND instance.

4. Run the following command in the same directory as your `docker-compose.yml` file to start the mint and a postgres database:

```bash
docker-compose up -d app database
```

## Development

### Setup rust

```bash
git clone https://github.com/ngutech21/moksha.git
cargo install just typos-cli sqlx-cli grcov wasm-pack wasm-opt
rustup component add llvm-tools-preview
cd moksha
```

### Install protobuf for your platform

This is needed for the LND backend.

```bash
sudo apt install protobuf-compiler
brew install protobuf
choco install protoc
```

### Config

```bash
mv .env.example .env
# edit .env file
vim .env
```

### Run mint (cashu-server)

To run the mint you need to setup a lightning regtest environment like [Polar](https://lightningpolar.com) and a Lnbits or Lnd instance. In Lnbits create a new wallet and copy the admin key into the .env file and set the url to your Lnbits instance. The mint uses PostgreSQL for storing used proofs and pending invoices. The database URL can be configured in the .env file.

```bash
install docker and docker-compose
docker compose up -d
just db-create
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

### Development

To run coverage, check for typos etc. use the just commands:

```bash
Available recipes:
    build-docker   # build the mint docker-image
    build-wasm     # compile all rust crates, that are relevant for the client, to wasm
    clean          # clean cargo
    db-create      # creates the postgres database
    db-migrate     # runs sqlx prepare
    db-prepare     # runs sqlx prepare
    default        # list all tasks
    deps           # install all dependencies
    final-check    # format code, check typos and run tests
    publish        # publish everything on crates.io
    run-cli *ARGS  # run cli-wallet with the given args
    run-coverage   # run coverage
    run-itests     # run integrationtests
    run-mint *ARGS # run the cashu-mint
    run-tests      # runs all tests
    typos          # check code for typos
    typos-fix-all  # fix all typos
```

## License

moksha is distributed under the terms of the MIT license.
See [LICENSE](LICENSE).
