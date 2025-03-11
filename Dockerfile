# Builder stage with Rust
FROM rust:1.85 as builder

# Install musl-tools for static compilation
RUN apt-get update && apt-get install -y musl-tools
RUN rustup target add aarch64-unknown-linux-musl

WORKDIR /usr/src/app

# Copy Cargo files first for better caching
COPY Cargo.toml Cargo.lock ./

# Create a dummy src/main.rs to build dependencies
RUN mkdir -p src && \
    echo "fn main() {println!(\"placeholder\")}" > src/main.rs && \
    cargo build --release --target aarch64-unknown-linux-musl && \
    rm -rf src

# Copy actual code and migrations
COPY src/ src/
COPY migrations/ migrations/

# Install sqlx-cli
RUN cargo install sqlx-cli --no-default-features --features postgres

# Build the application
RUN cargo build --release --target aarch64-unknown-linux-musl

# Alpine for lightweight and the static library
FROM alpine:latest

WORKDIR /usr/local/bin

# Copy the statically-linked binary from the builder stage 
COPY --from=builder /usr/src/app/target/aarch64-unknown-linux-musl/release/crud-hz-api .

# Copy migrations folder to the final image
COPY --from=builder /usr/src/app/migrations /usr/local/bin/migrations

# Make it executable
RUN chmod +x ./crud-hz-api

# Set the DATABASE_URL for runtime
ENV DATABASE_URL=postgres://postgres:postgres@db:5432/postgres

# Command to run the application
CMD ["./crud-hz-api"]