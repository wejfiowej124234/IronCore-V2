FROM rust:1.91-bookworm AS builder
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY . .

RUN cargo build --release --locked --all-features


FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates openssl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/ironcore /usr/local/bin/ironcore
COPY --from=builder /app/target/release/ironcore_migrate /usr/local/bin/ironcore_migrate

ENV BIND_ADDR=0.0.0.0:8088
EXPOSE 8088

CMD ["/usr/local/bin/ironcore"]
