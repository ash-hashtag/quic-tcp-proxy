FROM rust:slim AS builder

RUN apt-get update && apt-get install -y musl-tools \
 && rustup target add x86_64-unknown-linux-musl

WORKDIR /app

## don't copy certs, it is not safe, but for testing, no problem
COPY . . 

RUN cargo build --release --target x86_64-unknown-linux-musl

FROM alpine:latest
WORKDIR /app

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/quic-tcp-proxy .

## don't copy certs, it is not safe, but for testing, no problem
COPY --from=builder /app/certs ./certs

ENTRYPOINT ["./quic-tcp-proxy"]
