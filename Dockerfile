FROM rust:1.96-bookworm AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY migrations ./migrations
COPY src ./src
RUN cargo build --locked --release

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/sixty-six /usr/local/bin/sixty-six

ENV APP_ENV=production \
    DATABASE_URL=sqlite:///data/sixty-six.db \
    HOST=0.0.0.0 \
    PORT=3000

EXPOSE 3000
CMD ["/usr/local/bin/sixty-six"]

