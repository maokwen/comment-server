# builder
FROM docker.io/rust:slim-bullseye as builder
WORKDIR /workspace
COPY ./ .
RUN cargo install --path .

FROM docker.io/debian:bullseye-slim
WORKDIR /app
COPY --from=builder /workspace/target/release/comment-server /app/comment-server
COPY --from=builder /workspace/db /app/db
COPY --from=builder /workspace/Rocket.toml /app/Rocket.toml
CMD ["/app/comment-server"]

FROM docker.io/rust:slim-bullseye as builder
WORKDIR /workspace
COPY ./ .
RUN cargo install --path .

ENTRYPOINT [ "comment-server" ]
