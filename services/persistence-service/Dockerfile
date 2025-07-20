FROM rust:latest as builder

WORKDIR /app
COPY . .
RUN rm -f Cargo.lock
RUN cargo build --release --bin persistence-service

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/persistence-service /app/persistence-service
COPY --from=builder /app/services/persistence-service/migrations /app/migrations

EXPOSE 8005
CMD ["./persistence-service"]
