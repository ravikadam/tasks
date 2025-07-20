FROM rust:latest as builder

WORKDIR /app
COPY . .
RUN rm -f Cargo.lock
RUN cargo build --release --bin ai-agent-service

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/ai-agent-service /app/ai-agent-service

EXPOSE 8004
CMD ["./ai-agent-service"]
