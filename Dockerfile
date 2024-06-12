# 设置多阶段构建基础镜像
FROM rust:alpine as builder

# # 安装必要的工具和依赖
RUN apk add --no-cache pkgconfig build-base perl 

# 设置工作目录
WORKDIR /build

# 复制源码
COPY . .

RUN cargo build --release

# 使用Alpine镜像作为最终运行时环境
FROM alpine:latest
WORKDIR /app

# 根据平台选择对应的构建产物
COPY --from=builder /build/target/release/mikan-subscriber /app/mikan-subscriber
COPY entrypoint.sh /app/entrypoint.sh

RUN addgroup --gid 1000 subscribe && \
    adduser --uid 1000 --ingroup subscribe --disabled-password subscribe && \
    apk add --no-cache ca-certificates su-exec tzdata && \
    chown -R subscribe:subscribe /app && \
    chmod 755 /app/entrypoint.sh  && \
    chmod 755 /app/mikan-subscriber

ENTRYPOINT [ "/app/entrypoint.sh" ]
