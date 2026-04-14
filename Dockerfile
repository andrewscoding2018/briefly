FROM node:22-bookworm

ENV PNPM_HOME=/pnpm
ENV CARGO_HOME=/usr/local/cargo
ENV RUSTUP_HOME=/usr/local/rustup
ENV PATH="${PNPM_HOME}:${CARGO_HOME}/bin:${PATH}"
ENV CI=1

RUN apt-get update \
  && apt-get install -y --no-install-recommends build-essential ca-certificates curl pkg-config libssl-dev \
  && rm -rf /var/lib/apt/lists/*

RUN corepack enable \
  && corepack prepare pnpm@10.0.0 --activate

RUN curl https://sh.rustup.rs -sSf \
  | sh -s -- -y --profile minimal --default-toolchain stable \
  && "${CARGO_HOME}/bin/rustup" component add clippy rustfmt

WORKDIR /workspace

COPY package.json pnpm-lock.yaml pnpm-workspace.yaml Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY apps ./apps
COPY crates ./crates
COPY contracts ./contracts
COPY fixtures ./fixtures
COPY docs ./docs
COPY scripts ./scripts
COPY README.md ./

RUN pnpm install --frozen-lockfile

CMD ["./scripts/check"]
