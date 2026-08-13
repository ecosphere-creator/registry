# Changelog

## 1.0.1
- Backend now serves each LXS's docs bundle: `/api/lxs` cards gain
  `docs_available`; `/api/lxs/:name` gains a `docs` object with the version's
  README/api/examples/changelog/gotchas contents and `has_openapi`.
- Ships the `docs/` bundle (README, api, changelog, examples, openapi, gotchas).

## 1.0.0
- Initial release: clone/refresh `getecosphere/lxs-registry`, parse `lxs.yml`,
  serve `/api/lxs`, `/api/lxs/categories`, `/api/lxs/:name`.
