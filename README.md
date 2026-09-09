# palabot
The major part of my repositories has made with vibe coding, isn't professionals, only has made to learn and personal using, in my trying and eagerness to learn, my codes will have so fine practics to the extend that i study software engeniering and C, C++, Rust manuals. Thanks for understend with this apprentice of the development.

## Requisitos

- Docker (solo se usa para compilar el binario, no para ejecutarlo).
- `ffmpeg` instalado en el host (se usa en tiempo de ejecución para convertir el audio de Telegram).

## Configuración

Creá un archivo `.env` en la raíz del repo con tus credenciales:

```
TELOXIDE_TOKEN=tu_token_de_telegram
SERPAPI_KEY=tu_api_key_de_serpapi
```

## Compilación

El binario se compila dentro de un contenedor de Docker (para no depender de tener Rust y las
librerías de whisper.cpp instaladas en el host) y se exporta a `build_out/tsbpal`, junto con el
modelo de Whisper en `models/ggml-base.bin`:

```
cd ~/palabot
make
```

`make` corre por defecto los targets `check-ffmpeg`, `build` y `setup`: valida que `ffmpeg` esté
instalado, compila y exporta el binario/modelo vía Docker, y prepara `src/db.json` si no existe.
Si solo necesitás recompilar (por ejemplo, después de editar el código) sin repetir el chequeo de
`ffmpeg`, podés correr `make build`.

## Ejecución

```
cd ~/palabot
./run.sh
```

`run.sh` carga las variables de `.env`, exporta `WHISPER_MODEL_PATH` apuntando al modelo exportado
y arranca `build_out/tsbpal` con el token y la API key. Para dejarlo corriendo en segundo plano de
forma persistente, usá tu gestor de servicios preferido (systemd, `screen`, `tmux`, etc.) o
`nohup ./run.sh &`.

Para limpiar los artefactos de compilación (`build_out/`, `models/`), usá:

```
make clean
```
