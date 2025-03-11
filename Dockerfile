# Builder stage with Rust
FROM rust:1.85 as builder

# Install musl-tools for static compilation
RUN apt-get update && apt-get install -y musl-tools
RUN rustup target add aarch64-unknown-linux-musl

WORKDIR /usr/src/app
COPY . .

# Add the offline feature to your sqlx dependency in Cargo.toml
# This should be done before building
RUN sed -i 's/sqlx = { version = "[^"]*", features = \["postgres", "runtime-tokio-rustls"\]/sqlx = { version = "\0", features = ["postgres", "runtime-tokio-rustls", "offline", "migrations"]/g' Cargo.toml

# Set the DATABASE_URL environment variable for the build
ENV DATABASE_URL=postgres://postgres:postgres@localhost:5432/postgres

# Install sqlx-cli
RUN cargo install sqlx-cli --no-default-features --features postgres

# Generate sqlx-data.json file or copy one if you've pre-generated it
COPY /.sqlx/ .

# Build the application
RUN cargo build --release --target aarch64-unknown-linux-musl

# Alpine for lightweight and the static library
FROM alpine:latest

WORKDIR /usr/local/bin
# Copy the statically-linked binary from the builder stage 
COPY --from=builder /usr/src/app/target/aarch64-unknown-linux-musl/release/crud-hz-api .

# Make it executable
RUN chmod +x ./crud-hz-api

# Set the DATABASE_URL for runtime
ENV DATABASE_URL=postgres://postgres:postgres@postgresdb:5432/postgres

# Command to run the application
CMD ["./crud-hz-api"]