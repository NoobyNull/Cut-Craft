# CutCraft tool-database links

A Cloudflare Worker with a D1 database that remembers where each router-bit maker publishes
its current tool database (IDC Woodcraft, SpeTool, Amana, ...). It keeps **links and
fingerprints only**; the files stay on the makers' own sites.

- **Weekly check** (Mondays 14:00 UTC): opens each maker's page, finds the download link,
  downloads the file once to fingerprint it (SHA-256), and records a new version only when the
  fingerprint changes. A file re-uploaded under a new address is not a new version.
- **At most one check a day**, however it is started.
- A broken link keeps the last good one and shows why (link gone, download broken, page
  unreachable, site blocks automatic visitors).
- **Read-only to the public.** Changes need the admin token.

## Reading the list

| Address | What |
| --- | --- |
| `GET /` | Page listing every maker |
| `GET /v1/tool-databases` | All makers, JSON (`sources[]`: `id`, `vendor`, `format`, `url`, `filename`, `version`, `sha256`, `status`, `checked_at`, `changed_at`, `note`) |
| `GET /v1/tool-databases/<id>` | One maker |
| `GET /v1/tool-databases/<id>/history` | Every version seen, newest first |

`status` is `ok`, `manual`, `needs-link`, `new`, `blocked`, `link-missing`, `download-broken` or `page-unreachable`.

## Deploying (once)

```sh
cd services/tool-db-links
npm install
npx wrangler login
npx wrangler d1 create cutcraft-tool-db-links     # paste the database_id into wrangler.toml
npx wrangler d1 execute cutcraft-tool-db-links --remote --file schema.sql
npx wrangler d1 execute cutcraft-tool-db-links --remote --file seed.sql
npx wrangler secret put ADMIN_TOKEN               # a long random string; keep it private
npx wrangler deploy
```

## Admin

All need `Authorization: Bearer <ADMIN_TOKEN>`.

```sh
# check now (refused with 429 if the last check was under 24 hours ago)
curl -X POST -H "Authorization: Bearer $T" https://<worker>/admin/check

# set Amana's link by hand (amanatool.com blocks automatic visitors)
curl -X PUT -H "Authorization: Bearer $T" https://<worker>/admin/sources/amana-vectric/link \
  -d '{"url":"https://www.amanatool.com/.../file.vtdb","version":"12.25"}'

# add or change a maker
curl -X PUT -H "Authorization: Bearer $T" https://<worker>/admin/sources/whiteside-vectric \
  -d '{"vendor":"Whiteside","format":"vectric-vtdb","page_url":"https://...","link_match":"\\.vtdb"}'

# hide a maker
curl -X DELETE -H "Authorization: Bearer $T" https://<worker>/admin/sources/whiteside-vectric
```

`link_match` is a regular expression matched against every link on the maker's page;
`version_match` reads the version from the file name (first group). Leave it out when the maker
never changes the name (IDC); the fingerprint still catches new versions.

## Tests

`npm test` (Node 22 or newer; uses `node:sqlite` in place of D1, and a fake network).
