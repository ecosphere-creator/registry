# registry API

Base path: `/api`. Auth: none. Errors: `{ "error": "..." }` JSON with the
corresponding status code.

## Endpoints

### GET /api/lxs
- **Purpose:** list the latest version of every LXS, searchable and category-filterable.
- **Auth required:** no
- **Query params:** `category` (optional), `q` (optional, matches name/domain/summary)
- **Success 200:** array of cards, each with `name`, `version`, `category`,
  `status`, `publisher`, `summary`, `runtime`, `targets`, `source`, `commit`,
  and `docs_available` (whether the version ships a docs bundle).
- **Errors:** none (200 always, possibly empty).

```json
{
  "lxs": [
    {
      "name": "storage",
      "version": "1.0.5",
      "category": "Media",
      "status": "unverified",
      "publisher": "stuff8",
      "summary": "Photos & media: uploads, image/video processing, S3 storage, thumbnails",
      "runtime": "self-contained-static",
      "targets": ["linux/amd64"],
      "source": "https://github.com/getecosphere/storage.git",
      "commit": "abc123",
      "docs_available": true
    }
  ],
  "count": 1
}
```

### GET /api/lxs/categories
- **Purpose:** category names with LXS counts (latest versions only).
- **Auth required:** no
- **Success 200:**
```json
{ "categories": [ { "name": "Communication", "count": 3 }, { "name": "Media", "count": 1 } ] }
```
Categories sort by count desc, then name asc.

### GET /api/lxs/:name
- **Purpose:** full detail for the latest version of an LXS.
- **Auth required:** no
- **Success 200:** `name`, `domain`, `version`, `category`, `status`,
  `publisher`, `summary`, `targets`, `contract` (`version`, `api`, `db`,
  `env.required/optional`, `network.inbound/outbound`, `resources`), `runtime`
  (`base`, `libc`, `dependencies`), `provenance` (`source`, `commit`,
  `built_by`, `built_at`), `release`, `versions`, and `docs`:
```json
{
  "name": "storage",
  "version": "1.0.5",
  "docs": {
    "files": ["README.md", "api.md", "changelog.md", "examples.sh", "gotchas.md", "openapi.json"],
    "has_openapi": true,
    "index": "# storage — LXS docs\n...",
    "api": "# storage API\n...",
    "examples": "#!/usr/bin/env bash\n...",
    "changelog": "# Changelog\n...",
    "gotchas": "# Gotchas\n..."
  }
}
```
- **Errors:** `404` `{ "error": "LXS <name> not found" }` for an unknown name.

## Error reference
| Status | Body | When |
|---|---|---|
| 404 | `{ "error": "LXS <name> not found" }` | `/api/lxs/:name` unknown |

## Rate limiting / limits
None. The registry clone refreshes every 10 minutes from GitHub.
