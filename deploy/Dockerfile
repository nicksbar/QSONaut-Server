FROM node:20-alpine AS web-builder
WORKDIR /source/web
COPY web/package*.json ./
RUN npm ci
COPY web/ ./
RUN npm run build

FROM rust:1.93-bookworm AS rust-builder
WORKDIR /source
COPY Cargo.toml Cargo.lock* ./
COPY apps/ apps/
COPY crates/ crates/
COPY migrations/ migrations/
RUN cargo build --release --locked -p qsonaut-server

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=rust-builder /source/target/release/qsonaut-server /usr/local/bin/qsonaut-server
COPY --from=web-builder /source/web/build /app/web/build
ENV QSONAUT_SERVER_BIND=0.0.0.0:8080
ENV QSONAUT_SERVER_WEB_ROOT=/app/web/build
EXPOSE 8080
USER 65532:65532
ENTRYPOINT ["qsonaut-server"]
