FROM rust:1.75 as builder

WORKDIR /usr/src/app
COPY . .

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bullseye-slim

WORKDIR /usr/local/bin
COPY --from=builder /usr/src/app/target/release/crud-hz-api .

ARG DATABASE_URL
ENV DATABASE_URL=$DATABASE_URL

CMD ["./crud-hz-api"]