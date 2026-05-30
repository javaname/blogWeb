# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A personal blog system with a Go backend (Gin + GORM + SQLite) and a React admin panel (Vite + Semi UI). Includes an MCP server for AI-agent integration supporting both stdio and HTTP transports.

## Commands

```bash
# Backend
go build ./...                          # Build
go test ./...                           # Run all tests
go test ./internal/handler/...          # Run tests for a single package
go run . serve-web                      # Start web server (port 3000)
go run . serve-mcp --transport=stdio    # Start MCP server (stdio)
go run . serve-mcp --transport=http     # Start MCP server (HTTP)

# MCP token management
go run . mcp issue-token --name <name> --scopes <scopes> --transport http
go run . mcp revoke-token --name <name>

# Frontend (from client/ directory)
npm run dev                             # Dev server (port 5173, proxies /api to :3000)
npm run build                           # Production build → public/admin/
```

## Architecture

```
main.go                 Entry point with 3 subcommands: serve-web, serve-mcp, mcp
internal/
  app/bootstrap.go      DI container — wires all services, runs migrations, returns Application
  handler/http.go       All HTTP routes (Gin). Serves public blog + admin API
  middleware/           Auth, CSRF, security headers, request context
  model/               GORM models (User, Category, Article, MCPClient, MCPAuditLog)
  service/             Business logic layer (auth, articles, categories, likes, uploads, sessions, rate limiter, renderer)
  mcp/                 MCP server (tools, resources, prompts, transports, auth, audit)
  testutil/testapp.go  Test helpers: in-memory SQLite + miniredis, seed functions
config/config.go       YAML config loading
migrations/            SQL migration files (run sequentially on startup)
templates/             Go HTML templates for public-facing pages
client/                React admin SPA (Semi UI, react-router-dom v6)
public/                Static assets + admin SPA build output + uploads
```

## Key Design Decisions

- The backend serves both server-rendered HTML (public blog) and JSON API (admin). The React SPA is served as static files from `/admin`.
- SQLite is the sole database (file at `data/blog.db`). Redis handles sessions and rate limiting.
- Migrations are plain SQL files executed in order on every startup (idempotent via IF NOT EXISTS).
- Tests use `internal/testutil.NewApp()` which provides an isolated in-memory SQLite + miniredis instance per test.
- Image uploads are re-encoded server-side (strips metadata, validates content type).
- MCP server has scope-based authorization and audit logging. Supports `2025-11-25` protocol version.

## Configuration

Copy `config.example.yaml` to `config.yaml`. Required services: Redis (for sessions/rate limiting).

## Frontend Dev Workflow

The Vite dev server at `:5173` proxies `/api` requests to the Go backend at `:3000`. Run both simultaneously during development. Production builds output to `public/admin/` which the Go server serves directly.
