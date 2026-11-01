# Multi-stage Docker build for RemoteLite Relay & Signaling Server on Render / Cloud
FROM rust:1-slim as builder
WORKDIR /app

# Copy only the isolated server crate
COPY server ./server

# Build the lightweight signaling & relay server
WORKDIR /app/server
RUN cargo build --release

# Minimal Debian runtime container
FROM debian:bookworm-slim
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/server/target/release/remotelite-server /app/remotelite-server

# Render dynamically binds the PORT environment variable
ENV PORT=10000
EXPOSE 10000

CMD ["/app/remotelite-server"]
