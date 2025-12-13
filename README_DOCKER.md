# Docker for ZKClear

## Quick Start

### Production Build

```bash
# Build image
docker build -t zkclear-sequencer .

# Run via docker-compose
docker-compose up -d

# View logs
docker-compose logs -f zkclear-sequencer

# Stop
docker-compose down
```

### Development Environment

```bash
# Start dev container
docker-compose -f docker-compose.dev.yml up -d

# Enter container
docker-compose -f docker-compose.dev.yml exec zkclear-dev bash

# Inside container you can run:
cargo run -p zkclear-demo
cargo test
cargo check
```

## Configuration

### Environment Variables

Copy `.env.example` to `.env` and configure:

```bash
cp .env.example .env
```

Main parameters:
- `RUST_LOG`: Log level (error, warn, info, debug, trace)
- `DATA_DIR`: Directory for RocksDB data
- `MAX_QUEUE_SIZE`: Maximum transaction queue size
- `MAX_TXS_PER_BLOCK`: Maximum transactions per block
- `BLOCK_INTERVAL_SEC`: Block creation interval (seconds)
- `SEQUENCER_PORT`: Port for HTTP API

### Volumes

RocksDB data is stored in Docker volume `zkclear-data` for persistence between restarts.

## Structure

- `Dockerfile`: Production image (multi-stage build, optimized)
- `Dockerfile.dev`: Development image (with development tools)
- `docker-compose.yml`: Production configuration
- `docker-compose.dev.yml`: Development configuration
- `.dockerignore`: Exclusions for Docker build context

## Features

1. **RocksDB inside container**: Embedded database, no separate service required
2. **Multi-stage build**: Final image contains only necessary files
3. **Data volume**: Data persists between restarts
4. **Healthcheck**: Container health monitoring

## Future Improvements

- Add HTTP API service
- Add monitoring (Prometheus/Grafana)
- Add optional PostgreSQL for metadata
- Add backup/restore scripts
