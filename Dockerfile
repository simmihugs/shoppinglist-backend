# Build stage
FROM rust:bullseye AS builder

WORKDIR /usr/src/app

COPY . .
RUN cargo build --release

# Runtime stage
FROM debian:bullseye-slim AS runtime

RUN apt-get update && apt-get install -y \
    libsqlite3-0 \
    libssl1.1 && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/app/target/release/shoppinglist-backend /usr/local/bin/backend

EXPOSE 8080

CMD ["backend"]
