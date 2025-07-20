FROM rust:latest as builder

WORKDIR /app
COPY . .
RUN rm -f Cargo.lock
RUN cargo build --release --bin case-management-service

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/case-management-service /app/case-management-service

EXPOSE 8002
CMD ["./case-management-service"]
