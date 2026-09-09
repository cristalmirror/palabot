FROM rust:1-bookworm AS builder

RUN apt-get update \
    && apt-get install --yes --no-install-recommends \
        clang \
        cmake \
        libclang-dev \
        make \
        pkg-config \
        wget \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
RUN mkdir src \
    && printf 'fn main() {}\n' > src/main.rs \
    && cargo build --release \
    && rm -rf src

COPY src ./src
RUN touch src/main.rs src/transcribe.rs \
    && cargo build --release

RUN mkdir -p /app/models \
    && wget -q -O /app/models/ggml-base.bin \
        https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin

FROM scratch AS export
COPY --from=builder /app/target/release/tsbpal /build_out_tsbpal
COPY --from=builder /app/models/ggml-base.bin /ggml-base.bin