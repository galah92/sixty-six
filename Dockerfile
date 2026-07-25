# syntax=docker/dockerfile:1.7

FROM rust:1.96-bookworm AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY migrations ./migrations
COPY src ./src
RUN --mount=type=cache,id=sixty-six-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=sixty-six-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=sixty-six-release-target,target=/app/target,sharing=locked \
    cargo build --locked --release \
    && cp target/release/sixty-six /app/sixty-six

FROM debian:bookworm-slim

ARG BUILD_SHA=development

RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/sixty-six /usr/local/bin/sixty-six
COPY static/styles.css /app/static/styles.css

ENV APP_ENV=production \
    APP_VERSION=${BUILD_SHA} \
    DATABASE_URL=sqlite:///data/sixty-six.db \
    HOST=0.0.0.0 \
    PORT=3000 \
    STYLES_PATH=/app/static/styles.css

EXPOSE 3000
CMD ["/usr/local/bin/sixty-six"]
