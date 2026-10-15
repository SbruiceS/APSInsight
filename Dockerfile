# syntax=docker/dockerfile:1
# Reproducible build of apsi-server + apsi CLI + the Angular UI.
# No secrets are baked in: tokens are supplied at run time via APSI_TOKENS.

FROM node:24-bookworm-slim AS web
WORKDIR /src/web
COPY web/package.json web/package-lock.json ./
RUN npm ci --no-audit --no-fund
COPY web/ ./
RUN npm run build

FROM rust:1.99-bookworm AS rust
WORKDIR /src
COPY Cargo.toml Cargo.lock rustfmt.toml ./
COPY register ./register
COPY crates ./crates
RUN cargo build --release --locked -p apsi-server -p apsi-cli

FROM debian:bookworm-slim
RUN useradd --system --uid 10001 --home-dir /var/lib/apsinsight --shell /usr/sbin/nologin apsi \
 && mkdir -p /var/lib/apsinsight \
 && chown 10001 /var/lib/apsinsight
COPY --from=rust /src/target/release/apsi-server /usr/local/bin/apsi-server
COPY --from=rust /src/target/release/apsi /usr/local/bin/apsi
COPY --from=web /src/web/dist/web/browser /usr/share/apsinsight/web
USER 10001
ENV APSI_BIND=0.0.0.0:8080 \
    APSI_DATA_DIR=/var/lib/apsinsight \
    APSI_WEB_DIR=/usr/share/apsinsight/web \
    APSI_LOG_JSON=true
VOLUME ["/var/lib/apsinsight"]
EXPOSE 8080
ENTRYPOINT ["apsi-server"]
