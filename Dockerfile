# PLENUM release image (deploy unit, D1) — prod-only; dev never touches this.
# Three stages: build the SPA, build the Rust binaries, assemble a slim
# runtime. No .env, no key, no database password enters any layer — all
# configuration arrives as environment variables at run time.

# ── stage 1 · web: build the SPA ────────────────────────────────────────────
FROM node:20-slim AS web
WORKDIR /build/web
# Lockfile-first for layer caching; npm ci = reproducible from the lockfile.
COPY web/package.json web/package-lock.json ./
RUN npm ci --no-audit --no-fund
COPY web/ ./
RUN npm run build

# ── stage 2 · api: build the Rust binaries ──────────────────────────────────
# rust-toolchain.toml pins 1.95.0; the matching base image makes the pin a
# no-op instead of a mid-build toolchain download.
FROM rust:1.95-bookworm AS api
WORKDIR /build
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates/ crates/
COPY migrations/ migrations/
COPY .sqlx/ .sqlx/
# .sqlx is committed, so queries compile-check with no live database.
ENV SQLX_OFFLINE=true
RUN cargo build --release --bin api --bin seed

# ── stage 3 · runtime ───────────────────────────────────────────────────────
FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --create-home plenum
WORKDIR /app
COPY --from=api /build/target/release/api /app/api
COPY --from=api /build/target/release/seed /app/seed
# migrations ship for transparency/debug; both binaries embed them anyway.
COPY migrations/ /app/migrations/
COPY --from=web /build/web/dist/ /app/web/dist/
COPY docker/entrypoint.sh /app/entrypoint.sh
RUN chmod +x /app/entrypoint.sh && chown -R plenum:plenum /app
USER plenum
# The static tier's default path, made explicit and absolute.
ENV WEB_DIST=/app/web/dist
ENTRYPOINT ["/app/entrypoint.sh"]
