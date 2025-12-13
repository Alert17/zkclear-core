# Multi-stage build for optimized image size
FROM rust:1.75 as builder

WORKDIR /app

# Copy dependency files for caching
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build dependencies and project
# Use release profile for optimization
RUN cargo build --release -p zkclear-demo --features storage/rocksdb

# Final image
FROM debian:bookworm-slim

# Install required libraries for RocksDB
RUN apt-get update && apt-get install -y \
    libgcc-s1 \
    libc6 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy compiled binary
COPY --from=builder /app/target/release/zkclear-demo /app/zkclear-demo

# Create directory for RocksDB data
RUN mkdir -p /app/data

# Environment variables
ENV RUST_LOG=info
ENV DATA_DIR=/app/data

# Expose port (for future HTTP API)
EXPOSE 8080

# Run application
CMD ["./zkclear-demo"]

