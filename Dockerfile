# 设置多阶段构建基础镜像
FROM rust:latest as builder

# 安装必要的工具和依赖
RUN apt-get update && apt-get install -y musl-tools

# 设置工作目录
WORKDIR /build

# 复制源码
COPY . .
ARG TARGETARCH
# 为x86_64和aarch64目标添加musl工具链
RUN if [ "$TARGETARCH" = "amd64" ]; then rustup target add x86_64-unknown-linux-musl; else rustup target add aarch64-unknown-linux-musl; fi
# 编译
RUN if [ "$TARGETARCH" = "amd64" ]; then cargo build --release --target x86_64-unknown-linux-musl; else cargo build --release --target aarch64-unknown-linux-musl; fi

# 使用Alpine镜像作为最终运行时环境
FROM alpine:latest
WORKDIR /app

# 根据平台选择对应的构建产物
COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/mikan-subscriber /app/mikan-subscriber

RUN addgroup --gid 1000 subscribe && \
    adduser --uid 1000 --ingroup subscribe --disabled-password subscribe && \
    apk add --no-cache ca-certificates su-exec tzdata && \
    chown -R subscribe:subscribe /app && \
    chmod 755 /app/entrypoint.sh  && \
    chmod 755 /app/mikan-subscriber

ENTRYPOINT [ "/app/entrypoint.sh" ]
