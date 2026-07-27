# syntax=docker/dockerfile:1.7

FROM lukemathwalker/cargo-chef:latest-rust-1.92.0 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS cacher
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

FROM chef AS builder
COPY --from=cacher /app/target target
COPY --from=cacher /usr/local/cargo /usr/local/cargo
COPY . .
RUN cargo build --release --bin k1fix

FROM chef AS dev
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential \
        clang \
        lld \
        pkg-config \
        curl \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && cargo install cargo-nextest --locked \
    && cargo install cargo-llvm-cov --locked \
    && cargo install cargo-deny --locked \
    && cargo install cargo-watch --locked \
    && cargo install just --locked
WORKDIR /app
COPY . .
ENV CARGO_TERM_COLOR=always
ENV RUSTFLAGS="-C link-arg=-fuse-ld=lld"
CMD ["bash"]

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 10001 --shell /usr/sbin/nologin k1fix
COPY --from=builder /app/target/release/k1fix /usr/local/bin/k1fix
COPY fixtures /opt/k1fix/fixtures
USER k1fix
WORKDIR /work
ENTRYPOINT ["k1fix"]
