# build backend
FROM rust:1.71.0-slim-bullseye as rust-builder
RUN apt update && apt install -y make clang pkg-config libssl-dev protobuf-compiler

WORKDIR /rust-app
COPY . /rust-app  
RUN cargo build --package moksha-mint --release


FROM debian:bullseye-slim
COPY --from=rust-builder /rust-app/target/release/moksha-mint /

COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh
ENTRYPOINT ["/entrypoint.sh"]

WORKDIR /
CMD ["./moksha-mint"]