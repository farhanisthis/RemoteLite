# Deployment Guide

## Self-Hosted Infrastructure

OpenRemote's coordination server and TURN relay can be fully self-hosted.

### Requirements

- Docker and Docker Compose v2
- A server with a public IP address
- DNS records pointing to your server
- TLS certificates (Let's Encrypt recommended)

### Quick Start

```bash
cd infrastructure
cp env.example .env
# Edit .env with your domain and settings
docker compose up -d
```

### Services

| Service | Port | Description |
|---------|------|-------------|
| Coordination server | 8080 | Signaling and pairing |
| PostgreSQL | 5432 | Persistent storage (internal) |
| coturn | 3478/5349 | STUN/TURN relay |

### Configuration

See `infrastructure/env.example` for all configuration options.

## Managed Service

A managed coordination service will be available at `wss://coordination.openremote.dev` once the project reaches M6.

The managed service is optional. All infrastructure is compatible with self-hosted deployments.
