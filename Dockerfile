FROM rust:latest as builder

RUN apt-get update && apt-get install -y protobuf-compiler libprotobuf-dev && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml build.rs /usr/src/app/
COPY src /usr/src/app/src/
COPY proto /usr/src/app/proto/
COPY sql /usr/src/app/sql
WORKDIR /usr/src/app
RUN cargo build --release

FROM debian:bookworm-slim as runner
RUN apt-get update && apt-get install -y protobuf-compiler libprotobuf-dev && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/app/target/release/rs-clickhouse-writer /
COPY --from=builder /usr/src/app/sql /

CMD ["/rs-clickhouse-writer"]
