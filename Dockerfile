FROM rust:alpine as builder

WORKDIR /build
# ENV TARGET_CC=x86_64-linux-musl-gcc

RUN apk add --no-cache musl-dev musl-utils musl-dev gcc clang18-dev openssl-dev  g++ llvm18-dev


COPY . .
RUN cargo build --release

FROM alpine:latest
WORKDIR /app
COPY entrypoint.sh /app
COPY --from=builder /build/target/release/mikan-subscriber /app

RUN addgroup --gid 1000 subscribe && \
    adduser --uid 1000 --ingroup subscribe --disabled-password subscribe && \
    apk add --no-cache ca-certificates su-exec tzdata && \
    chown -R subscribe:subscribe /app && \
    chmod 755 /app/entrypoint.sh 

CMD [ "/app/entrypoint.sh" ]



