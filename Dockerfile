FROM rust:1.54 as cargo
COPY Cargo.toml Cargo.lock ./
RUN cargo fetch

FROM rust:1.54 as build
WORKDIR /usr/src/app
COPY --from=dependencies /usr/local/cargo /usr/local/cargo
COPY . .
RUN cargo install

FROM chromedp/headless-shell:latest
RUN apt-get update && apt-get install -y dumb-init openssl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/hcloud-project-manager /bin/hcloud-project-manager
ENTRYPOINT ["dumb-init", "--"]
CMD ["/bin/hcloud-project-manager"]
