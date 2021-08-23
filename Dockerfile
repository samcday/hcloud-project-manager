FROM rust:1.54 as cargo
WORKDIR /usr/src/app
COPY Cargo.toml Cargo.lock ./
RUN mkdir src
RUN echo "fn main() {}" > src/main.rs
RUN cargo build

FROM rust:1.54 as build
WORKDIR /usr/src/app
COPY --from=cargo /usr/local/cargo /usr/local/cargo
COPY . .
RUN cargo install --path .

FROM chromedp/headless-shell:latest
RUN apt-get update && apt-get install -y dumb-init openssl && rm -rf /var/lib/apt/lists/*
COPY --from=build /usr/local/cargo/bin/hcloud-project-manager /bin/hcloud-project-manager
ENTRYPOINT ["dumb-init", "--"]
CMD ["/bin/hcloud-project-manager"]
