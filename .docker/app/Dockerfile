FROM alpine:3.18 as base

FROM rust:1 as builder
RUN rustup target add x86_64-unknown-linux-musl
RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
RUN cargo binstall -y sccache --locked --install-path /usr/local/bin
RUN apt-get update && apt-get install -y musl-tools && rm -rf /var/lib/apt/lists/*
WORKDIR /usr/src/app
RUN cargo init
RUN echo "fn main() {println!(\"if you see this, the build broke\")}" > src/main.rs && \
  touch -d @0 src/main.rs
COPY ./Cargo.lock ./Cargo.toml ./
ENV RUSTC_WRAPPER=/usr/local/bin/sccache
ENV SCCACHE_DIR=/root/.cache/sccache
RUN --mount=type=cache,target=/root/.cache/sccache \
  cargo fetch --target=x86_64-unknown-linux-musl && \
  cargo build --target=x86_64-unknown-linux-musl --release && \
  rm -rf src
COPY . .
RUN --mount=type=cache,target=/root/.cache/sccache \
  touch src/main.rs && \
  cargo install --target=x86_64-unknown-linux-musl --path .

FROM base as runner
WORKDIR /usr/src/app
COPY --from=builder /usr/local/cargo/bin/telegram-bot-api-proxy /usr/local/bin/telegram-bot-api-proxy
CMD ["telegram-bot-api-proxy"]
