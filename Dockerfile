FROM rust:1.54 as deps
WORKDIR /usr/src/app
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release && cargo clean --release -p hcloud-project-manager && rm src/main.rs && rmdir src

FROM deps as builder
COPY . .
RUN cargo build --release

FROM chromedp/headless-shell:94.0.4606.12
RUN apt-get update && apt-get install -y dumb-init openssl ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/app/target/release/hcloud-project-manager /bin/hcloud-project-manager
COPY actions-*.sh /
ENTRYPOINT ["dumb-init", "--"]
CMD ["/bin/hcloud-project-manager"]
