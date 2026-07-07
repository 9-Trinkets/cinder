FROM rust:1-bookworm AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY cinder-core ./cinder-core
COPY cinder-tui ./cinder-tui
COPY cinder-desk ./cinder-desk
COPY cinder-srv ./cinder-srv

RUN cargo build --release -p cinder-srv

FROM debian:bookworm-slim
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/cinder-srv /usr/local/bin/cinder-srv
COPY content ./content
COPY config ./config
COPY neuron.toml ./neuron.toml

EXPOSE 3000

CMD ["cinder-srv"]
