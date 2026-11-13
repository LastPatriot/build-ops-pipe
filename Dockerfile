# 1. Base stage - runtime image with a locked-down non-root user
FROM debian:bookworm-slim AS base

ARG USERNAME=appuseradm
ARG USER_UID=1000
ARG USER_GID=1000

# git + docker CLI are runtime deps: this binary shells out to both.
# safe.directory: CI clones the repo as a different owner than appuseradm.
RUN groupadd --gid $USER_GID $USERNAME && \
    useradd --uid $USER_UID --gid $USER_GID --create-home --shell /bin/bash $USERNAME && \
    mkdir -p /app/logs && \
    chown -R $USERNAME:$USERNAME /app/logs && \
    apt-get update && apt-get install -y --no-install-recommends ca-certificates git && \
    rm -rf /var/lib/apt/lists/* && \
    git config --system --add safe.directory '*'

COPY --from=docker:27.3.1-cli /usr/local/bin/docker /usr/local/bin/docker

WORKDIR /app

# 2. Build stage
FROM rust:1-bookworm AS build
WORKDIR /app

# Cache dependency compilation separately from source changes
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

COPY src ./src
RUN touch src/main.rs && cargo build --release

# 3. Final stage (uses the base with your user configured)
FROM base AS final
ARG USERNAME=appuseradm
WORKDIR /app

COPY --from=build --chown=$USERNAME:$USERNAME /app/target/release/build-ops-pipe ./build-ops-pipe

# CLI used as a pipeline step image: no port; on PATH so steps can call it from the clone dir.
ENV PATH="/app:${PATH}"

# Switch to your custom user
USER $USERNAME

# Configuration is injected at runtime (env vars / --env-file / secrets),
# never baked into the image - do not COPY .env in here.
CMD ["build-ops-pipe"]
