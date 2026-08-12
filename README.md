# LXS Registry domain

Browse and categorize the versioned Linux Services in
[getecosphere/lxs-registry](https://github.com/getecosphere/lxs-registry).

- `backend/` — Rust **Actix** API: clones/refreshes the registry repo, parses
  every `lxs.yml`, and serves `/api/lxs` (list + search + category filter),
  `/api/lxs/categories`, and `/api/lxs/:name` (detail).
- `frontend/` — Rust **Leptos** (SSR) browse UI: category chips, search, cards,
  and per-LXS detail with contract + provenance + source link.

## Run

```bash
# backend
cd backend && cargo run --release   # PORT=8260, LXS_REGISTRY_CACHE=/var/lib/lxs-registry
# frontend
cd frontend && cargo run --release  # PORT=8261
```

Compose into an estate with `ecompose.yml`:

```yaml
services:
  lxs-backend:
    path: registry/backend
    runtimes: [rust]
  lxs-frontend:
    path: registry/frontend
    runtimes: [rust]
```

The gateway routes `/api/lxs/*` to the backend.
