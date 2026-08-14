# registry — LXS docs

## Capability

Browses and categorizes the public LXS registry (`getecosphere/lxs-registry`).
A **single binary** serves both the browse UI (SSR homepage at `/` and detail at
`/lxs/:name`) and the JSON API of every versioned Linux Service: latest-version
cards with category counts, plus a per-name detail view that includes the full
contract, runtime, provenance, release history, **and the docs bundle shipped
with each version**. If you need to list/search LXS packages, inspect their
contracts/docs, or browse the registry as a website, this is the LXS.

## What it owns / never owns

- **Owns:** reading + refreshing the registry git clone, parsing `lxs.yml`,
  latest-version aggregation, category counts, serving docs contents, rendering
  the browse UI (SSR).
- **Never owns:** the binaries/manifests themselves, publishing/auth, or any
  persistence — it is stateless.

## Compose it

```yaml
# ecompose.yml
services:
  lxs-registry:
    lxs: registry@1.0.2
    grants:
      secrets: [SERVER_PORT, LXS_REGISTRY_CACHE]
    shared_tools: [git]   # runtime dependency for clone/refresh
```

## Quick usage

```sh
# browse UI (SSR)
curl http://127.0.0.1:8261/

# list latest version of every LXS (cards include docs_available)
curl http://127.0.0.1:8261/api/lxs

# detail for one LXS — contract + runtime + provenance + docs bundle
curl http://127.0.0.1:8261/api/lxs/storage

# category counts
curl http://127.0.0.1:8261/api/lxs/categories
```

## Docs index

- `api.md` — full endpoint reference with request/response JSON
- `examples.sh` — executable smoke test (golden request→response pairs)
- `openapi.json` — machine-readable OpenAPI 3.0 spec
- `changelog.md` — version history + breaking changes
- `gotchas.md` — production-learned constraints and operational gotchas

## For AI agents

This LXS is distributed as a **binary only** — these docs are the entire
interface. Match `api.md` shapes exactly; run `examples.sh` against a pulled
binary or live estate URL before trusting behavior.
