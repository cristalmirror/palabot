# Build stage: compile the Rust bot and whisper.cpp bindings.
FROM rust:1-bookworm AS builder

RUN apt-get update \
    && apt-get install --yes --no-install-recommends \
        clang \
        cmake \
        libclang-dev \
        make \
        pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy manifests first so dependency compilation can be cached by Docker.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src \
    && printf 'fn main() {}\n' > src/main.rs \
    && cargo build --release \
    && rm -rf src

COPY src ./src
RUN cargo build --release

# Runtime stage: ffmpeg is required to convert Telegram audio to 16 kHz WAV.
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install --yes --no-install-recommends \
        ca-certificates \
        ffmpeg \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/tsbpal /app/tsbpal

# The model is intentionally mounted from the host instead of copied into the image.
ENV WHISPER_MODEL_PATH=/app/models/ggml-base.bin
VOLUME ["/app/models"]

ENTRYPOINT ["/app/tsbpal"]
