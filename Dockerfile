FROM rust:1.98.1-bookworm AS builder
WORKDIR /build
COPY rust-toolchain.toml Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
COPY fixtures ./fixtures
RUN cargo build --release \
    && mkdir -p /out \
    && (cp target/release/aurum /out/aurum 2>/dev/null || cp target/release/aurum-rs /out/aurum)

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates wget \
    && rm -rf /var/lib/apt/lists/*
RUN useradd --create-home --uid 10001 aurum
COPY --from=builder /out/aurum /usr/local/bin/aurum
COPY migrations /app/migrations
COPY fixtures /app/fixtures
RUN mkdir -p /app /data /backups \
    && chown -R aurum:aurum /app /data /backups
USER aurum
WORKDIR /app
EXPOSE 8080
ENTRYPOINT ["aurum"]
CMD ["server"]