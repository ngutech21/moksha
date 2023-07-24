# build backend
FROM rust:1.68.0-slim-bullseye as rust-builder
RUN apt update && apt install -y make clang pkg-config libssl-dev
WORKDIR /rust-app
COPY . /rust-app  
ARG SQLX_OFFLINE=true
RUN cargo build --workspace --package moksha-mint --release


FROM debian:bullseye-slim
COPY --from=rust-builder /rust-app/target/release/moksha-mint /

WORKDIR /
CMD ["./moksha-mint"]