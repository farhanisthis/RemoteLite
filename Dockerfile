# Multi-stage Docker build for RemoteLite Relay & Signaling Server on Render / Cloud
FROM rust:1.80-slim as builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin remotelite-server

FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/remotelite-server /app/remotelite-server

ENV PORT=10000
EXPOSE 10000
CMD ["/app/remotelite-server"]
