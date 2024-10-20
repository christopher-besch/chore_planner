# one of:
#  amd64
#  arm64
ARG TARGETARCH=amd64

#########
# amd64 #
#########
FROM rust AS builder_amd64

RUN apt-get update && apt-get install -y musl-tools musl-dev
RUN rustup target add x86_64-unknown-linux-musl && rustup component add clippy

WORKDIR /usr/src/chore_planner

# cache dependencies
COPY ./Cargo.toml ./Cargo.toml
RUN echo 'fn main() {}' > dummy.rs && \
    sed -i 's#src/main.rs#dummy.rs#' Cargo.toml
RUN cargo clippy -- -D warnings && \
    cargo test && \
    cargo build --release --target x86_64-unknown-linux-musl

# actually build
COPY ./Cargo.toml ./Cargo.toml
COPY ./src ./src
RUN cargo clippy -- -D warnings && \
    cargo test && \
    cargo build --release --target x86_64-unknown-linux-musl && \
    cp /usr/src/chore_planner/target/x86_64-unknown-linux-musl/release/chore_planner /var/run/chore_planner



#########
# arm64 #
#########
FROM messense/rust-musl-cross:aarch64-musl AS builder_arm64

RUN rustup target add aarch64-unknown-linux-musl && rustup component add clippy

WORKDIR /usr/src/chore_planner

# cache dependencies
COPY ./Cargo.toml ./Cargo.toml
RUN echo 'fn main() {}' > dummy.rs && \
    sed -i 's#src/main.rs#dummy.rs#' Cargo.toml
RUN cargo clippy -- -D warnings && \
    cargo test && \
    cargo build --release --target aarch64-unknown-linux-musl

# actually build
COPY ./Cargo.toml ./Cargo.toml
COPY ./src ./src
RUN cargo clippy -- -D warnings && \
    cargo test && \
    cargo build --release --target aarch64-unknown-linux-musl && \
    cp /usr/src/chore_planner/target/aarch64-unknown-linux-musl/release/chore_planner /var/run/chore_planner


###############
# final image #
###############
FROM builder_${TARGETARCH} AS builder

FROM --platform=linux/${TARGETARCH} alpine
COPY --from=builder /var/run/chore_planner /var/run/chore_planner
WORKDIR /var/run
ENTRYPOINT ["/var/run/chore_planner"]

LABEL org.opencontainers.image.source=https://github.com/christopher-besch/chore_planner
