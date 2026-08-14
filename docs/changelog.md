# Changelog

## 1.0.3
- Fix: an empty `LXS_REGISTRY_CACHE` env value now falls back to the default
  `/var/lib/lxs-registry` instead of scanning the empty string (which made the
  API return zero packages when `.env` declared `LXS_REGISTRY_CACHE=`).

## 1.0.2
- **Single combined binary**: the browse UI (Leptos SSR) and the JSON API are
  now one LXS — `/` and `/lxs/:name` render the UI, `/api/*` serves the API, all
  same-origin on one `SERVER_PORT`. The old split backend/frontend pair is gone;
  compose this one service.
- Removed the separate Actix `backend/` crate (its `/api/*` routes moved into
  the frontend axum app).

## 1.0.1
- Backend now serves each LXS's docs bundle: `/api/lxs` cards gain
  `docs_available`; `/api/lxs/:name` gains a `docs` object with the version's
  README/api/examples/changelog/gotchas contents and `has_openapi`.
- Ships the `docs/` bundle (README, api, changelog, examples, openapi, gotchas).

## 1.0.0
- Initial release: clone/refresh `getecosphere/lxs-registry`, parse `lxs.yml`,
  serve `/api/lxs`, `/api/lxs/categories`, `/api/lxs/:name`.
