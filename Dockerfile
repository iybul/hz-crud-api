# Builder stage with Rust
FROM rust:1.85 as builder

# Install musl-tools for static compilation
RUN apt-get update && apt-get install -y musl-tools
RUN rustup target add aarch64-unknown-linux-musl

WORKDIR /usr/src/app
COPY . .


RUN cargo install sqlx-cli
CMD export DATABASE_URL=postgres://postgres:postgres@postresdb:5432/postgres
RUN cargo sqlx prepare
# support for cross-compiling to aarch64-unknown-linux-musl target
# no dependency on glibc

RUN cargo build --release --target aarch64-unknown-linux-musl


# Alpine for lightweight and the static library
FROM alpine:latest

WORKDIR /usr/local/bin
# Copy the statically-linked binary from the builder stage 
COPY --from=builder /usr/src/app/target/aarch64-unknown-linux-musl/release/crud-hz-api .

# Make it executable
RUN chmod +x ./crud-hz-api

# Command to run the application
CMD ["./crud-hz-api"]