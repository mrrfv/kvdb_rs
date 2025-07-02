# Use the official Rust image as the build environment
FROM rust:1.72 as builder

WORKDIR /app

# Copy Cargo.toml and Cargo.lock first to cache dependencies
COPY Cargo.toml Cargo.lock ./
RUN mkdir src
RUN echo "fn main() {}" > src/main.rs
RUN cargo fetch

# Copy the source code
COPY . .

# Build the release binary
RUN cargo build --release

# Use a minimal base image
FROM debian:bullseye-slim

WORKDIR /app

# Copy the compiled binary from the builder
COPY --from=builder /app/target/release/kvdb_rs /app/kvdb_rs

# Set the startup command
CMD ["./kvdb_rs"]