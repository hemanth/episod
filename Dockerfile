# Build stage
FROM rust:1.85-slim-bookworm AS builder

WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates curl && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/episod /usr/local/bin/episod

EXPOSE 8080
VOLUME ["/data"]

ENV EPISOD_DB=/data/episod.db

ENTRYPOINT ["episod"]
CMD ["dev", "--port", "8080", "--db", "/data/episod.db"]
