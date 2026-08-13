#!/usr/bin/env bash
# registry LXS smoke test — golden request→response pairs.
# Usage: BASE_URL=<http://host:port> ./examples.sh
set -euo pipefail
BASE_URL="${BASE_URL:-http://127.0.0.1:8260}"

echo "# registry LXS smoke test -> $BASE_URL"

code=$(curl -s -o /tmp/reg-list.json -w '%{http_code}' "$BASE_URL/api/lxs")
test "$code" = "200" || { echo "FAIL /api/lxs -> $code"; exit 1; }
echo "OK /api/lxs -> 200"

python3 - "$BASE_URL" <<'PY' || { echo "FAIL /api/lxs shape"; exit 1; }
import json, sys, urllib.request
data = json.load(urllib.request.urlopen(sys.argv[1] + "/api/lxs"))
assert "lxs" in data and isinstance(data["lxs"], list)
assert all("name" in m and "version" in m and "docs_available" in m for m in data["lxs"])
PY
echo "OK /api/lxs shape (cards carry docs_available)"

code=$(curl -s -o /tmp/reg-cats.json -w '%{http_code}' "$BASE_URL/api/lxs/categories")
test "$code" = "200" || { echo "FAIL /api/lxs/categories -> $code"; exit 1; }
echo "OK /api/lxs/categories -> 200"

for name in auth chat email-manager notifications profile slides storage registry; do
  code=$(curl -s -o /tmp/reg-detail.json -w '%{http_code}' "$BASE_URL/api/lxs/$name")
  test "$code" = "200" || { echo "FAIL /api/lxs/$name -> $code"; exit 1; }
  python3 - "$name" <<'PY' || { echo "FAIL /api/lxs/$name docs"; exit 1; }
import json, sys, urllib.request
name = sys.argv[1]
data = json.load(urllib.request.urlopen(f"{sys.argv[2]}/api/lxs/{name}"))
assert data.get("name") == name
if data.get("docs"):
    assert data["docs"].get("index") and data["docs"].get("api")
PY
  echo "OK /api/lxs/$name -> 200 (name matches, docs present if bundled)"
done

code=$(curl -s -o /tmp/reg-404.json -w '%{http_code}' "$BASE_URL/api/lxs/definitely-not-a-real-lxs")
test "$code" = "404" || { echo "FAIL unknown name -> $code (expected 404)"; exit 1; }
echo "OK unknown LXS -> 404"

echo "ALL CHECKS PASSED"
