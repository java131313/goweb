# Apple 记录本（apple-notes）

Apple 备忘录风格的全栈记录本应用。

- **后端**：Rust + Rocket + SQLite（rusqlite bundled）
- **前端**：React 18 + antd 5 + axios + Vite（前后端分离，开发时 Vite 代理 `/api`）
- **交付**：多阶段 Dockerfile 构建单镜像，GitHub Actions 推送至 GHCR（ghcr.io）

## 目录结构

```
├── backend/            # Rocket 后端
│   ├── Cargo.toml
│   └── src/main.rs     # REST API + 静态文件托管 + CORS
├── frontend/           # React 前端
│   ├── src/App.jsx     # 记录本主界面（列表 + 编辑器，自动保存）
│   └── src/api.js      # axios 实例
├── Dockerfile          # 多阶段构建：前端 → 后端 → 运行时
├── docker-compose.yml  # 本地运行（可选）
└── .github/workflows/docker.yml  # CI：构建镜像并推送 ghcr.io
```

## API

| 方法 | 路径 | 说明 |
|---|---|---|
| GET | `/api/notes?query=` | 列表，支持标题/内容模糊搜索 |
| GET | `/api/notes/<id>` | 详情 |
| POST | `/api/notes` | 新建 `{title, content}` |
| PUT | `/api/notes/<id>` | 更新 `{title, content}` |
| DELETE | `/api/notes/<id>` | 删除 |

## 本地开发

```bash
# 终端 1：后端（http://localhost:8000）
cd backend && cargo run

# 终端 2：前端（http://localhost:5173，/api 已代理到 8000）
cd frontend && npm install && npm run dev
```

## 生产模式（单容器）

Rocket 同时托管 API 与前端构建产物（`static/` 目录）：

```bash
cd frontend && npm run build
cp -r dist/* ../backend/static/   # 目录不存在先创建
cd ../backend && DATABASE_URL=notes.db cargo run --release
# 打开 http://localhost:8000
```

## Docker

```bash
docker build -t apple-notes .
docker run -d -p 8000:8000 -v notes-data:/data --name apple-notes apple-notes
```

## GitHub Actions → GHCR

推送到 `go-back` 或 `main` 分支即触发 workflow（也可手动 `workflow_dispatch`），
自动构建镜像并推送到：

```
ghcr.io/java131313/goweb:latest
ghcr.io/java131313/goweb:<分支名>
ghcr.io/java131313/goweb:sha-<commit>
```

部署到服务器后拉取运行：

```bash
echo <GITHUB_PAT> | docker login ghcr.io -u <用户名> --password-stdin
docker run -d -p 8000:8000 -v notes-data:/data ghcr.io/java131313/goweb:latest
```

## 环境变量

| 变量 | 默认值 | 说明 |
|---|---|---|
| `DATABASE_URL` | `notes.db` | SQLite 文件路径（容器内 `/data/notes.db`） |
| `ROCKET_ADDRESS` | `0.0.0.0`（容器内） | 监听地址 |
| `ROCKET_PORT` | `8000` | 监听端口 |
| `VITE_API_BASE` | 空（同源） | 前端 axios 的 baseURL，构建时注入 |
