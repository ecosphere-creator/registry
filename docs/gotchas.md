# Gotchas

Production constraints that are NOT visible in the binary — from code and the
domain README.md.

- **`git` is a runtime dependency.** The backend clones/refreshes the registry
  repo (`LXS_REGISTRY_CACHE`). If `git` is missing, startup refresh fails
  (logged, not fatal) and the API serves an empty list until it recovers.
- **First startup clones over the network.** `refresh_registry` does a depth-1
  clone from `https://github.com/getecosphere/lxs-registry.git` on first boot —
  the cache dir must be writable and the host must reach GitHub. On subsequent
  starts it runs `git pull --ff-only` inside the cache; a non-fast-forward pull
  fails silently and the last good manifests stay in memory.
- **Refresh cadence:** every 10 minutes. A freshly published LXS may take up to
  10 minutes to appear.
- **Detail endpoint reads docs from disk** at `cache/<name>/<version>/docs/`.
  Older versions published before the docs contract existed have no docs dir →
  `docs` is `null` and the UI shows the "no docs bundle" fallback.
- **No auth, no persistence, no rate limiting.** Treat it as a read-only public
  browse API; it is stateless by design.
