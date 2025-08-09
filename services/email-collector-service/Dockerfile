FROM rust:latest as builder

# The context of the build is the repository root.  Copy everything so
# Cargo can resolve workspace members and shared crates.
WORKDIR /app
COPY . .

# Remove any existing Cargo.lock to ensure the workspace is rebuilt
RUN rm -f Cargo.lock

# Build the release binary for the email collector service
RUN cargo build --release --bin email-collector-service

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/email-collector-service /app/email-collector-service

EXPOSE 8006

CMD ["./email-collector-service"]