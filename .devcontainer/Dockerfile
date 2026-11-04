FROM rust:1.99.1-bookworm

ARG LEAN_TOOLCHAIN=leanprover/lean4:v4.35.0
ARG NODE_VERSION=26.13.0

RUN apt-get update \
    && apt-get install -y --no-install-recommends sqlite3 libsqlite3-dev xz-utils curl git \
    && rm -rf /var/lib/apt/lists/*

RUN rustup component add rustfmt clippy \
    && rustup target add wasm32-unknown-unknown \
    && cargo install wasm-bindgen-cli --version 0.2.130 --locked \
    && rm -rf /usr/local/cargo/registry

# Node.js (official binary tarball).
RUN ARCH=$(dpkg --print-architecture | sed 's/amd64/x64/') \
    && curl -fsSL "https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-linux-${ARCH}.tar.xz" \
       | tar -xJ -C /usr/local --strip-components=1 \
    && node --version && npm --version

# Lean 4 via elan, installed system-wide so any user can run it.
ENV ELAN_HOME=/usr/local/elan
ENV PATH=/usr/local/elan/bin:$PATH
RUN curl -sSfL https://raw.githubusercontent.com/leanprover/elan/master/elan-init.sh \
       | sh -s -- -y --no-modify-path --default-toolchain "${LEAN_TOOLCHAIN}" \
    && chmod -R a+rwX "${ELAN_HOME}" \
    && lean --version

WORKDIR /workspace
