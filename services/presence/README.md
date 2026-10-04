# Presence: "playing now"

A Cloudflare Worker that counts the openOMSI games being played right now. A running game
posts `/ping` every three minutes and `/bye` when it ends (`crates/omsi-app/src/presence.rs`,
setting `presence`, on by default); a session counts for ten minutes after its last ping.
What it gets is a random id made for that session, the game's version and the kind of system -
no name, and no address is stored.

- `GET /players` - `{"players": 12, "systems": {"windows": 9, "linux": 2, "android": 1}, "updated": "..."}`
  (the website reads it)
- `GET /badge` - the count for a [shields.io endpoint badge](https://shields.io/badges/endpoint-badge)
  (the README shows it)

## Deploy

Cloudflare's Workers Builds deploys it from this repository on every push to `main`
(Worker `openomsi`, root directory `services/presence`, deploy command `npx wrangler deploy`).
By hand: `npx wrangler login` once, then `npx wrangler deploy` in this folder.

It runs on Cloudflare's free plan (a Worker and one SQLite-backed Durable Object). The address
`wrangler deploy` prints goes into `SERVICE` in `crates/omsi-app/src/presence.rs`, the badge in
`README.md` and `PRESENCE` in `site/app.js`.
