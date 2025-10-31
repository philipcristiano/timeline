FROM rust:1.91-bookworm as builder
WORKDIR /usr/src/app

COPY . .
COPY --from=d3fk/tailwindcss:stable /tailwindcss /usr/local/bin/tailwindcss
ENV SQLX_OFFLINE=true
RUN mkdir -p /usr/src/output
RUN cargo build --release --target-dir=/usr/src/output

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y procps ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/output/release/timeline* /usr/local/bin/

ENTRYPOINT ["/usr/local/bin/timeline"]
