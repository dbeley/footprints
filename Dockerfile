# Build stage
FROM rust:1.75 as builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src
COPY templates ./templates

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install required runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/target/release/footprints /app/footprints
COPY --from=builder /app/templates /app/templates

# Create directory for database
RUN mkdir -p /data

# Set environment variables
ENV DATABASE_PATH=/data/footprints.db
ENV PORT=3000

# Expose port
EXPOSE 3000

# Run the application
CMD ["/app/footprints"]
