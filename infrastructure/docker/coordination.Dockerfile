# Multi-stage build for OpenRemote Coordination Server
# Build stage
FROM rust:1.82-slim-bookworm AS builder

WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files for dependency caching
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates/ ./crates/
COPY services/ ./services/
COPY apps/windows/host-service/Cargo.toml ./apps/windows/host-service/
COPY apps/windows/session-agent/Cargo.toml ./apps/windows/session-agent/

# Build only the coordination server in release mode
RUN cargo build --release --bin openremote-coordination

# Runtime stage
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /build/target/release/openremote-coordination ./

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

ENTRYPOINT ["./openremote-coordination"]
