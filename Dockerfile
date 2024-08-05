# syntax=docker/dockerfile:1
ARG RUST_VERSION=1.80.0
ARG APP_NAME=dnstracer
FROM rust:${RUST_VERSION}-slim-bullseye AS build
ARG APP_NAME
RUN apt-get update && apt-get install -y pkg-config libssl-dev
WORKDIR /usr/src/dnstracer
COPY . .
RUN cargo build --release
RUN cp ./target/release/$APP_NAME /bin/dnstracer

FROM debian:bullseye-slim AS final
RUN apt-get update && apt-get install -y pkg-config libssl-dev

ARG UID=10001
RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    rpf
USER rpf

COPY --from=build /bin/dnstracer /bin/

CMD ["/bin/dnstracer"]