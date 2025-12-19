# Multi-stage build for optimized image size
FROM rust:1.75 as builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy dependency files for caching
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build dependencies and project
# Use release profile for optimization
RUN cargo build --release -p zkclear-api --features rocksdb

# Final image
FROM debian:bookworm-slim

# Install required libraries for RocksDB and runtime
RUN apt-get update && apt-get install -y \
    libgcc-s1 \
    libc6 \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for security
RUN useradd -m -u 1000 zkclear && \
    mkdir -p /app/data && \
    chown -R zkclear:zkclear /app

WORKDIR /app

# Copy compiled binary
COPY --from=builder /app/target/release/zkclear-api /app/zkclear-api

# Set ownership
RUN chown zkclear:zkclear /app/zkclear-api && \
    chmod +x /app/zkclear-api

# Switch to non-root user
USER zkclear

# Environment variables
ENV RUST_LOG=info
ENV DATA_DIR=/app/data
ENV STORAGE_PATH=/app/data
ENV BLOCK_INTERVAL_SEC=1
ENV MAX_QUEUE_SIZE=10000
ENV MAX_TXS_PER_BLOCK=100

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Expose port
EXPOSE 8080

# Run application
CMD ["./zkclear-api"]

