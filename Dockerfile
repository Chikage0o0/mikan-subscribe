FROM rust:latest as builder

WORKDIR /build
# ENV TARGET_CC=x86_64-linux-musl-gcc
ADD . /build
RUN apt update && apt install -y musl-tools && rustup target add x86_64-unknown-linux-musl
RUN cargo build --release --target x86_64-unknown-linux-musl

FROM alpine:latest
WORKDIR /app
COPY entrypoint.sh /app
COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/bin /app

RUN addgroup --gid 1000 subscribe && \
    adduser --uid 1000 --ingroup subscribe --disabled-password subscribe && \
    apk add --no-cache ca-certificates su-exec tzdata && \
    chown -R subscribe:subscribe /app && \
    chmod 755 /app/entrypoint.sh 

CMD [ "/app/entrypoint.sh" ]



