# palabot
The major part of my repositories has made with vibe coding, isn't professionals, only has made to learn and personal using, in my trying and eagerness to learn, my codes will have so fine practics to the extend that i study software engeniering and C, C++, Rust manuals. Thanks for understend with this apprentice of the development.

## To use:

``
 cd ~/palabot

  docker build -t palabot:local .

  docker run -d \
    --name palabot \
    --restart unless-stopped \
    -v "$PWD/models:/app/models:ro" \
    palabot:local \
    TU_TOKEN_DE_TELEGRAM \
    TU_API_KEY_DE_SERPAPI
``