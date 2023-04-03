FROM rust:1.68-slim-bullseye as base
RUN apt-get update && apt install -y --no-install-recommends ca-certificates
RUN cargo install cargo-chef
WORKDIR /app

FROM base AS cargo-chef
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM base AS cargo-chef-builder
RUN apt-get update && apt install -y --no-install-recommends pkg-config libssl-dev
COPY --from=cargo-chef /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

FROM debian:bullseye-slim AS application
RUN apt-get update && apt install -y --no-install-recommends ca-certificates

# Image Server

FROM cargo-chef-builder AS image-server-build
COPY . .
RUN cargo build --release --bin image-server

FROM application AS image-server
COPY --from=image-server-build /app/target/release/image-server ./pokecord-image-server
CMD ["./pokecord-image-server"]

