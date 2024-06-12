# 多平台构建的基础镜像
FROM ghcr.io/cross-rs/x86_64-unknown-linux-musl:latest as builder-x86_64
FROM ghcr.io/cross-rs/aarch64-unknown-linux-musl:latest as builder-aarch64

# 设置构建环境并构建x86_64架构的二进制
FROM builder-x86_64 as x86_64-builder
WORKDIR /build
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && \
    . $HOME/.cargo/env && \
    rustup target add x86_64-unknown-linux-musl

COPY . .
RUN . $HOME/.cargo/env && cargo build --release --target x86_64-unknown-linux-musl

# 设置构建环境并构建aarch64架构的二进制
FROM builder-aarch64 as aarch64-builder
WORKDIR /build
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && \
    . $HOME/.cargo/env && \
    rustup target add aarch64-unknown-linux-musl

COPY . .
RUN . $HOME/.cargo/env && cargo build --release --target aarch64-unknown-linux-musl

# 使用Alpine镜像作为最终运行时环境
FROM alpine:latest
WORKDIR /app

# 根据平台选择对应的构建产物
ARG TARGETARCH
COPY entrypoint.sh /app
COPY --from=x86_64-builder /build/target/x86_64-unknown-linux-musl/release/mikan-subscriber /app/mikan-subscriber-x86_64
COPY --from=aarch64-builder /build/target/aarch64-unknown-linux-musl/release/mikan-subscriber /app/mikan-subscriber-aarch64
RUN if [ "$TARGETARCH" = "amd64" ]; then mv /app/mikan-subscriber-x86_64 /app/mikan-subscriber; else mv /app/mikan-subscriber-aarch64 /app/mikan-subscriber; fi && \
    rm -f /app/mikan-subscriber-x86_64 /app/mikan-subscriber-aarch64

RUN addgroup --gid 1000 subscribe && \
    adduser --uid 1000 --ingroup subscribe --disabled-password subscribe && \
    apk add --no-cache ca-certificates su-exec tzdata && \
    chown -R subscribe:subscribe /app && \
    chmod 755 /app/entrypoint.sh 

ENTRYPOINT [ "/app/entrypoint.sh" ]
