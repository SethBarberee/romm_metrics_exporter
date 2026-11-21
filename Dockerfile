FROM rust:slim-bookworm AS builder

WORKDIR /usr/src/app

RUN apt-get update && apt-get install -y musl-tools
RUN rustup target add x86_64-unknown-linux-musl

COPY . .

RUN cargo build --target x86_64-unknown-linux-musl --release

FROM alpine:latest

COPY --from=builder /usr/src/app/target/x86_64-unknown-linux-musl/release/romm_metrics_exporter /romm_metrics_exporter 

CMD ["/romm_metrics_exporter"]
