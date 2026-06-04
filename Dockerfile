FROM rust:1.88-slim AS builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim

WORKDIR /app
COPY --from=builder /app/target/release/limited /usr/local/bin/limited

EXPOSE 8080
CMD ["limited"]
