FROM rust:latest as builder

WORKDIR /app
COPY . .
RUN rm -f Cargo.lock
RUN cargo build --release --bin channel-service

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/channel-service /app/channel-service

EXPOSE 8001
CMD ["./channel-service"]
