# 多架构 Dockerfile（使用预编译的二进制文件）
ARG TARGETARCH

# 运行时阶段 - 使用最小的基础镜像
FROM debian:bookworm-slim

# 声明构建参数
ARG TARGETARCH

# 安装运行时依赖
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# 创建非root用户
RUN useradd -m -u 1000 appuser

# 设置工作目录
WORKDIR /app

# 根据目标架构复制对应的二进制文件
COPY binaries/drinkup-image-${TARGETARCH} /app/drinkup-image

# 设置执行权限并更改文件所有者
RUN chmod +x /app/drinkup-image && \
    chown appuser:appuser /app/drinkup-image

# 切换到非root用户
USER appuser

# 暴露端口
EXPOSE 3000

# 设置环境变量
ENV RUST_LOG=info

# 健康检查
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

# 启动应用
CMD ["./drinkup-image"] 