# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A personal blog system with a Rust backend (Axum + SQLx + SQLite) and a React admin panel (Vite + Semi UI). Includes an MCP server for AI-agent integration supporting both stdio and HTTP transports.

## Commands

```bash
# Backend
cargo build --offline                   # Build
cargo test --offline                    # Run all Rust tests
cargo run -- db migrate --apply -config config.yaml
cargo run -- serve-web -config config.yaml
cargo run -- serve-mcp -transport stdio -config config.yaml
cargo run -- serve-mcp -transport http -config config.yaml

# MCP token management
cargo run -- mcp issue-token -config config.yaml -name <name> -scopes <scopes> -transport http
cargo run -- mcp revoke-token -config config.yaml -name <name>

# Frontend (from client/ directory)
npm run dev                             # Dev server (port 5173, proxies /api to :3000)
npm run build                           # Production build → public/admin/
```

## Architecture

```
src/main.rs             CLI entry point: serve-web, serve-mcp, mcp, db
src/app.rs              HTTP router and response contract
src/http_public.rs      Public SSR pages and public API
src/admin_*.rs          Admin auth, read APIs, write APIs
src/mcp.rs              MCP server tools, resources, prompts, transports, auth, audit
src/db.rs               SQLite migration check/apply
src/config.rs           YAML config loading
migrations/             SQL migration files
templates/              Historical public page templates
client/                React admin SPA (Semi UI, react-router-dom v6)
public/                Static assets + admin SPA build output + uploads
tests/                 Rust integration tests and frozen golden fixtures
```

## Key Design Decisions

- The backend serves both server-rendered HTML (public blog) and JSON API (admin). The React SPA is served as static files from `/admin`.
- SQLite is the sole database (file at `data/blog.db`). Redis handles sessions and rate limiting.
- Migrations are plain SQL files. Server startup checks migration state and does not apply migrations implicitly.
- Tests use Rust integration helpers and frozen `tests/golden/**/*.json` compatibility fixtures.
- Image uploads validate content type and size before saving.
- MCP server has scope-based authorization and audit logging. Supports `2025-11-25` protocol version.

## Configuration

Copy `config.example.yaml` to `config.yaml`. Required services: Redis (for sessions/rate limiting).

## Frontend Dev Workflow

The Vite dev server at `:5173` proxies `/api` requests to the Rust backend at `:3000`. Run both simultaneously during development. Production builds output to `public/admin/` which the Rust server serves directly.
