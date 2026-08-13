# registry

The LXS Registry domain — browse and categorize versioned Linux Services from
the `getecosphere/lxs-registry` repository.

## Docs

This LXS ships as a binary only; `docs/` is the entire consumer interface.

- `docs/README.md` — agent-facing index: capability, ownership, compose, quick usage
- `docs/api.md` — full endpoint reference (methods, paths, JSON, errors)
- `docs/changelog.md` — version history
- `docs/examples.sh` — executable smoke test
- `docs/openapi.json` — machine-readable OpenAPI 3.0.3 spec
- `docs/gotchas.md` — production constraints

## What this LXS owns

- Reading and categorizing the LXS registry (`getecosphere/lxs-registry` repo):
  cloning/refreshing it, parsing every `lxs.yml`, and serving a browse API
  (`/api/lxs`, `/api/lxs/categories`, `/api/lxs/:name`).
- The latest-version-per-name aggregation, semver ordering, and category counts.
- The docs bundle contents of each published LXS version (`docs/*` files served
  in the detail response).

## What this LXS must NEVER own

- The binaries or manifests themselves — they live in the registry repo.
- Publishing/authenticating LXS — that is the `eco lxs` CLI's job.
- Any persistence — this domain is stateless; the git clone is a cache
  (`LXS_REGISTRY_CACHE`).

## Contracts (public API)

- `GET /api/lxs` — list latest version of each LXS (search `q`, category filter);
  each card includes `docs_available`
- `GET /api/lxs/categories` — category counts
- `GET /api/lxs/:name` — detail: contract, runtime, provenance, versions, and the
  full `docs` bundle (index, api, examples, changelog, gotchas, has_openapi)

## Environment variables

- `LXS_REGISTRY_CACHE` — dir holding the git clone of the registry (default `/var/lib/lxs-registry`)
- `PORT` — listen port (default `8260`)

## Runtime

- Rust (Actix 4), self-contained static binary (musl), `git` runtime dependency
  for clone/refresh. Refreshes the registry clone every 10 minutes.
