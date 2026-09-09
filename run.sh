#!/usr/bin/env bash
set -e

cd "$(dirname "$0")"

if [ -f .env ]; then
  export $(grep -v '^#' .env | xargs)
fi

if ! command -v ffmpeg &> /dev/null; then
    echo "Error: ffmpeg no está instalado en tu sistema Debian."
    echo "Instálalo ejecutando: sudo apt install ffmpeg"
    exit 1
fi

export WHISPER_MODEL_PATH="$PWD/models/ggml-base.bin"
export RUST_LOG="${RUST_LOG:-info}"

echo "Iniciando palabot..."
./build_out/tsbpal "$TELOXIDE_TOKEN" "$SERPAPI_KEY"