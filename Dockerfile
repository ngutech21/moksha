# build backend
FROM rust:1.74.1-slim-bookworm as rust-builder
RUN apt update && apt install -y make clang pkg-config libssl-dev protobuf-compiler

WORKDIR /rust-app
COPY . /rust-app  
RUN cargo build --package moksha-mint --release


FROM alpine:3.19.0
COPY --from=rust-builder /rust-app/target/release/moksha-mint /

COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh
ENTRYPOINT ["/entrypoint.sh"]

WORKDIR /
CMD ["./moksha-mint"]