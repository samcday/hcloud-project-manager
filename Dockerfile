FROM rust:1.54 as deps
WORKDIR /usr/src/app
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release && cargo clean --release -p hcloud-project-manager && rm src/main.rs && rmdir src

FROM deps as builder
COPY . .
RUN cargo build --release

FROM gcr.io/distroless/cc-debian10
COPY --from=builder /usr/src/app/target/release/hcloud-project-manager /bin/hcloud-project-manager
COPY actions-*.sh /
ENTRYPOINT ["/bin/hcloud-project-manager"]
