FROM rust:1.77-bookworm as builder
WORKDIR /usr/src/app

COPY Cargo.toml Cargo.lock /usr/src/app/
COPY --from=d3fk/tailwindcss:stable /tailwindcss /usr/local/bin/tailwindcss
RUN \
    mkdir /usr/src/app/src && \
    echo 'fn main() {}' > /usr/src/app/src/main.rs && \
    cargo build --release && \
    rm -Rvf /usr/src/app/src

COPY . .
RUN touch src/main.rs
RUN cargo build --release -v
RUN cargo install --path .

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y procps ca-certificates && rm -rf /var/lib/apt/lists/*
COPY atlas.hcl /
COPY schema /schema
COPY --from=builder /usr/local/cargo/bin/timeline /usr/local/bin/timeline

COPY --from=arigaio/atlas:0.16.0-community /atlas /atlas

ENTRYPOINT ["/usr/local/bin/timeline"]
