FROM rust:1.40 as builder
WORKDIR /usr/src/myapp
COPY . .
RUN cargo install --path .

FROM chromedp/headless-shell:latest
RUN apt-get update && apt-get install -y dumb-init openssl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/hcloud-project-manager /bin/hcloud-project-manager
ENTRYPOINT ["dumb-init", "--"]
CMD ["/bin/hcloud-project-manager"]
