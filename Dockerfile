# Builder stage with Rust
FROM rust:1.85 as builder

# Install musl-tools for static compilation
RUN apt-get update && apt-get install -y musl-tools
RUN rustup target add aarch64-unknown-linux-musl

WORKDIR /usr/src/app

# Copy everything to the build context
COPY . .

# Install sqlx-cli
RUN cargo install sqlx-cli --no-default-features --features postgres

# Build the application directly
RUN cargo build --release --target aarch64-unknown-linux-musl

# Print contents of the target directory to verify the binary exists
RUN ls -la target/aarch64-unknown-linux-musl/release/

# Alpine for lightweight and the static library
FROM alpine:latest

WORKDIR /usr/local/bin

# Copy the statically-linked binary from the builder stage
# Make sure this binary name matches your actual binary
COPY --from=builder /usr/src/app/target/aarch64-unknown-linux-musl/release/crud-hz-api .

# Copy migrations folder to the final image
COPY --from=builder /usr/src/app/migrations /usr/local/bin/migrations

# Run migration
RUN sqlx migrate add create_tables

# Make it executable
RUN chmod +x ./crud-hz-api

# Set the DATABASE_URL for runtime
ENV DATABASE_URL=postgres://postgres:postgres@db:5432/postgres

# For debugging, print current directory contents
RUN ls -la

# Add explicit output to show the container is starting
CMD echo "Starting application" && ./crud-hz-api