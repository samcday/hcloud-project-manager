# syntax = docker/dockerfile:1.2

FROM rust:1.54 as builder
WORKDIR /usr/src/app
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/src/app/target \
    cargo build --release && cp target/release/hcloud-project-manager /bin

FROM debian:buster-slim
RUN apt-get update && apt-get install -y openssl ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /bin/hcloud-project-manager /bin/hcloud-project-manager
COPY actions-*.sh /
ENTRYPOINT ["/bin/hcloud-project-manager"]
