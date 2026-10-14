# ---------- 阶段 1：构建前端 ----------
FROM node:20-alpine AS frontend
WORKDIR /build/frontend
COPY frontend/package.json frontend/package-lock.json* ./
RUN npm install
COPY frontend/ ./
RUN npm run build

# ---------- 阶段 2：构建后端 ----------
FROM rust:1-bookworm AS backend
WORKDIR /build/backend
COPY backend/Cargo.toml backend/Cargo.lock* ./
COPY backend/src ./src
RUN cargo build --release

# ---------- 阶段 3：运行时 ----------
FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=backend /build/backend/target/release/apple-notes ./apple-notes
COPY --from=frontend /build/frontend/dist ./static

ENV ROCKET_ADDRESS=0.0.0.0 \
    ROCKET_PORT=8000 \
    DATABASE_URL=/data/notes.db

# SQLite 数据持久化目录
VOLUME ["/data"]
EXPOSE 8000

CMD ["./apple-notes"]
