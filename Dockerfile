# Builder stage with Rust
FROM rust:1.75 as builder

# Install musl-tools for static compilation
RUN apt-get update && apt-get install -y musl-tools
RUN rustup target add aarch64-unknown-linux-musl

WORKDIR /usr/src/app
COPY . .

# Build a static binary using musl
RUN cargo build --release --target aarch64-unknown-linux-musl

# Final lightweight stage
FROM alpine:latest

WORKDIR /usr/local/bin
# Copy the statically-linked binary from the builder stage
COPY --from=builder /usr/src/app/target/aarch64-unknown-linux-musl/release/crud-hz-api .

# Make it executable
RUN chmod +x ./crud-hz-api

# Command to run the application
CMD ["./crud-hz-api"]