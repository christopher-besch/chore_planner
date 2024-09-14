FROM rust AS builder

RUN apt-get update && apt-get install -y musl-tools musl-dev
RUN rustup target add x86_64-unknown-linux-musl && rustup component add clippy

WORKDIR /usr/src/chore_planner
COPY ./Cargo.toml ./Cargo.toml
COPY ./src ./src

RUN cargo clippy -- -D warnings && \
    cargo test && \
    cargo build --release --target x86_64-unknown-linux-musl

FROM alpine
COPY --from=builder /usr/src/chore_planner/target/x86_64-unknown-linux-musl/release/chore_planner /var/run/chore_planner
WORKDIR /var/run
ENTRYPOINT ["/var/run/chore_planner"]

LABEL org.opencontainers.image.source=https://github.com/christopher-besch/chore_planner
