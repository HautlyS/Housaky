FROM rust:1.75-slim as builder

WORKDIR /build
COPY . .

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/housaky /usr/local/bin/

EXPOSE 8080

ENTRYPOINT ["housaky"]
CMD ["--port", "8080"]
