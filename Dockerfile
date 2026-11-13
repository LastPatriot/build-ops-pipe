# 1. Build stage - static musl binary
FROM rust:1-alpine AS build
RUN apk add --no-cache musl-dev
WORKDIR /app

# Cache dependency compilation separately from source changes
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

COPY src ./src
RUN touch src/main.rs && cargo build --release

# 2. Final stage - Alpine runtime with a locked-down non-root user
FROM alpine:3.20 AS final

ARG USERNAME=appuseradm
ARG USER_UID=1000
ARG USER_GID=1000

# git is a runtime dep: this binary shells out to it.
# No Docker CLI here: Bitbucket mounts its own at /usr/local/bin/docker into every pipe
# container (the calling step needs `services: docker`).
# safe.directory: CI clones the repo as a different owner than appuseradm.
RUN addgroup -g $USER_GID $USERNAME && \
    adduser -D -u $USER_UID -G $USERNAME -s /bin/sh $USERNAME && \
    mkdir -p /app/logs && \
    chown -R $USERNAME:$USERNAME /app/logs && \
    apk add --no-cache ca-certificates git && \
    git config --system --add safe.directory '*'

WORKDIR /app

COPY --from=build --chown=$USERNAME:$USERNAME /app/target/release/build-ops-pipe ./build-ops-pipe

# CLI used as a pipeline step image: no port; on PATH so steps can call it from the clone dir.
ENV PATH="/app:${PATH}"

USER $USERNAME

# Configuration is injected at runtime (env vars / --env-file / secrets),
# never baked into the image - do not COPY .env in here.
CMD ["build-ops-pipe"]
