# builder
FROM docker.io/rust:alpine as builder
WORKDIR /workspace
RUN apk add --no-cache musl-dev sqlite

# build deps
COPY Cargo.toml Cargo.toml
RUN mkdir src/
RUN echo "fn main() {println!(\"if you see this, the build broke\")}" > src/main.rs
RUN cargo build --release
RUN rm -f /workspace/target/release/deps/comment_server*

# build apps
COPY . .
RUN cargo build --release

# runner
FROM docker.io/alpine:latest
RUN addgroup -g 1000 app
RUN adduser -D -s /bin/sh -u 1000 -G app app

WORKDIR /app
COPY --from=builder /workspace/target/release/comment-server /app/comment-server
COPY --from=builder /workspace/db /app/db
COPY --from=builder /workspace/Rocket.toml /app/Rocket.toml

RUN chown -R app:app /app
USER app
CMD ["/app/comment-server"]
