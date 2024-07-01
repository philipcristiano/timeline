FROM rust:1.79-bookworm as builder
WORKDIR /usr/src/app

COPY . .
COPY --from=d3fk/tailwindcss:stable /tailwindcss /usr/local/bin/tailwindcss
RUN cargo install --path .

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y procps ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/cargo/bin/timeline-migrate /usr/local/bin/timeline-migrate
COPY --from=builder /usr/local/cargo/bin/timeline /usr/local/bin/timeline

ENTRYPOINT ["/usr/local/bin/timeline"]
