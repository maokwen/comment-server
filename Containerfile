# builder
FROM docker.io/rust:latest as builder
WORKDIR /workspace

# MUSL Libc
RUN apt-get update
RUN apt-get install musl-tools -y
RUN rustup target add x86_64-unknown-linux-musl

# build deps
COPY Cargo.toml Cargo.toml
RUN mkdir src/
RUN echo "fn main() {println!(\"if you see this, the build broke\")}" > src/main.rs
RUN RUSTFLAGS=-Clinker=musl-gcc cargo build --release --target=x86_64-unknown-linux-musl
RUN rm -f target/release/deps/comment-server*

# build apps
COPY . .
RUN RUSTFLAGS=-Clinker=musl-gcc cargo build --release
RUN cargo install --path .

# runner
FROM docker.io/alpine:latest
RUN addgroup -g 1000 comment-server
RUN adduser -D -s /bin/sh -u 1000 -G comment-server comment-server

WORKDIR /app
COPY --from=builder /workspace/target/release/comment-server /app/comment-server
COPY --from=builder /workspace/db /app/db
COPY --from=builder /workspace/Rocket.toml /app/Rocket.toml

RUN chown comment-server:comment-server comment-server
USER comment-server
CMD ["/app/comment-server"]
