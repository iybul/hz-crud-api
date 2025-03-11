# Build stage
FROM rust:1.85 as builder

WORKDIR /app

# accept the build argument
ARG DATABASE_URL

ENV DATABASE_URL=postgres://postgres:postgres@localhost:5432/postgres

COPY . . 

RUN cargo build --release

# Production stage
FROM debian:buster-slim

WORKDIR /usr/local/bin

COPY --from=builder /app/target/release/rust-crud-api .

ENV DATABASE_URL=postgres://postgres:postgres@localhost:5432/postgres

CMD ["./crud-hz-api"]