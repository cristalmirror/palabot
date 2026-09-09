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
        wget \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/tsbpal /app/tsbpal

# The model is downloaded during the image build instead of being committed to git.
RUN mkdir -p /app/models \
    && wget -q -O /app/models/ggml-base.bin \
        https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin \
    && test -s /app/models/ggml-base.bin

ENV WHISPER_MODEL_PATH=/app/models/ggml-base.bin

ENTRYPOINT ["/app/tsbpal"]
