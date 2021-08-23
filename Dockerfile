FROM rust:1.54 as builder
WORKDIR /usr/src/app
COPY Cargo.toml Cargo.lock ./
RUN mkdir .cargo
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo vendor > .cargo/config && rm src/main.rs && rmdir src

COPY . .
RUN cargo install --path .

FROM chromedp/headless-shell:latest
RUN apt-get update && apt-get install -y dumb-init openssl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/hcloud-project-manager /bin/hcloud-project-manager
ENTRYPOINT ["dumb-init", "--"]
CMD ["/bin/hcloud-project-manager"]
