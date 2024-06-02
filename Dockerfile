FROM  ghcr.io/cross-rs/x86_64-unknown-linux-musl:latest as builder

WORKDIR /build

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && \
    . $HOME/.cargo/env && \
    rustup target add x86_64-unknown-linux-musl




COPY . .
RUN . $HOME/.cargo/env && cargo build --release --target x86_64-unknown-linux-musl

FROM alpine:latest
WORKDIR /app
COPY entrypoint.sh /app
COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/mikan-subscriber /app

RUN addgroup --gid 1000 subscribe && \
    adduser --uid 1000 --ingroup subscribe --disabled-password subscribe && \
    apk add --no-cache ca-certificates su-exec tzdata && \
    chown -R subscribe:subscribe /app && \
    chmod 755 /app/entrypoint.sh 

ENTRYPOINT [ "/app/entrypoint.sh" ]



