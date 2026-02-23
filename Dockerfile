# syntax=docker/dockerfile:1

# ── Stage 1: Build ────────────────────────────────────────────
FROM rust:1.93-slim-trixie@sha256:9663b80a1621253d30b146454f903de48f0af925c967be48c84745537cd35d8b AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# 1. Copy manifests to cache dependencies
COPY Cargo.toml Cargo.lock ./
# Create dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo build --release --locked
RUN rm -rf src

# 2. Copy source code
COPY . .
# Touch main.rs to force rebuild
RUN touch src/main.rs
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo build --release --locked && \
    strip target/release/housaky

# ── Stage 2: Permissions & Config Prep ───────────────────────
FROM busybox:1.37@sha256:b3255e7dfbcd10cb367af0d409747d511aeb66dfac98cf30e97e87e4207dd76f AS permissions
# Create directory structure (simplified workspace path)
RUN mkdir -p /housaky-data/.housaky /housaky-data/workspace

# Create minimal config for PRODUCTION (allows binding to public interfaces)
# NOTE: Provider configuration must be done via environment variables at runtime
RUN cat > /housaky-data/.housaky/config.toml << 'EOF'
workspace_dir = "/housaky-data/workspace"
config_path = "/housaky-data/.housaky/config.toml"
api_key = ""
default_provider = "openrouter"
default_model = "arcee-ai/trinity-large-preview:free"
default_temperature = 0.7

[gateway]
port = 3000
host = "[::]"
allow_public_bind = true
EOF

RUN chown -R 65534:65534 /housaky-data

# ── Stage 3: Development Runtime (Debian) ────────────────────
FROM debian:trixie-slim@sha256:f6e2cfac5cf956ea044b4bd75e6397b4372ad88fe00908045e9a0d21712ae3ba AS dev

# Install runtime dependencies + basic debug tools
RUN apt-get update && apt-get install -y \
    ca-certificates \
    openssl \
    curl \
    git \
    iputils-ping \
    vim \
    && rm -rf /var/lib/apt/lists/*

COPY --from=permissions /housaky-data /housaky-data
COPY --from=builder /app/target/release/housaky /usr/local/bin/housaky

# Overwrite minimal config with DEV template (Ollama defaults)
COPY dev/config.template.toml /housaky-data/.housaky/config.toml
RUN chown 65534:65534 /housaky-data/.housaky/config.toml

# Environment setup
# Use consistent workspace path
ENV HOUSAKY_WORKSPACE=/housaky-data/workspace
ENV HOME=/housaky-data
# Defaults for local dev (Ollama) - matches config.template.toml
ENV PROVIDER="ollama"
ENV HOUSAKY_MODEL="llama3.2"
ENV HOUSAKY_GATEWAY_PORT=3000

# Note: API_KEY is intentionally NOT set here to avoid confusion.
# It is set in config.toml as the Ollama URL.

WORKDIR /housaky-data
USER 65534:65534
EXPOSE 3000
ENTRYPOINT ["housaky"]
CMD ["gateway", "--port", "3000", "--host", "[::]"]

# ── Stage 4: Production Runtime (Distroless) ─────────────────
FROM gcr.io/distroless/cc-debian13:nonroot@sha256:84fcd3c223b144b0cb6edc5ecc75641819842a9679a3a58fd6294bec47532bf7 AS release

COPY --from=builder /app/target/release/housaky /usr/local/bin/housaky
COPY --from=permissions /housaky-data /housaky-data

# Environment setup
ENV HOUSAKY_WORKSPACE=/housaky-data/workspace
ENV HOME=/housaky-data
# Default provider (model is set in config.toml, not here,
# so config file edits are not silently overridden)
ENV PROVIDER="openrouter"
ENV HOUSAKY_GATEWAY_PORT=3000

# API_KEY must be provided at runtime!

WORKDIR /housaky-data
USER 65534:65534
EXPOSE 3000
ENTRYPOINT ["housaky"]
CMD ["gateway", "--port", "3000", "--host", "[::]"]
