.PHONY: all build check-ffmpeg setup clean

all: check-ffmpeg build setup

check-ffmpeg:
	@which ffmpeg > /dev/null 2>&1 || ( \
		echo "==> [ALERTA] ffmpeg no está instalado en tu Debian host."; \
		echo "==> Ejecuta: sudo apt update && sudo apt install -y ffmpeg"; \
		exit 1 \
	)

build:
	@echo "==> Limpiando permisos anteriores..."
	@mkdir -p build_out models
	@echo "==> Compilando y exportando desde Docker..."
	DOCKER_BUILDKIT=1 docker build --target export --output type=local,dest=./tmp_export .
	@mv ./tmp_export/build_out_tsbpal ./build_out/tsbpal
	@mv ./tmp_export/ggml-base.bin ./models/ggml-base.bin
	@rm -rf ./tmp_export
	@chmod +x build_out/tsbpal
	@echo "==> Exportación finalizada con éxito."

setup:
	@mkdir -p src
	@if [ ! -f src/db.json ]; then \
		echo '{"users":[]}' > src/db.json; \
		echo "==> Archivo src/db.json preparado."; \
	fi

clean:
	@rm -rf build_out models tmp_export
	@echo "==> Limpieza completada."