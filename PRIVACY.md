# Privacy

Steam Manifest Downloader can optionally send **anonymous usage statistics**
so the maintainer can see which features are worth investing in and which
are broken. This document describes exactly what is sent, what is **not**
sent, and how you can opt in or out.

## Telemetry is opt-in

On first launch the app shows a dialog asking whether you want to help.
**Nothing is transmitted unless you click "Yes, help out."** Declining is the
default if you close the dialog. You can change your choice anytime in
**Settings → Advanced Settings → Anonymous Usage Statistics**.

## What is collected

When telemetry is enabled, the app sends small events describing:

- App version, build channel (`stable` / `dev` / `dev-local`), OS, architecture
- A random install UUID generated once on first accept (no link to your
  identity — this is just so two events from the same session can be
  correlated)
- A random session UUID regenerated every time the app starts
- **Event counters** for a fixed list of actions:
  - `app_start` — the app was launched
  - `settings_opened` — you opened the Settings dialog
  - `theme_toggled` — you switched between dark / light theme
  - `search_performed` — you ran a search in Step 1
  - `lua_parsed` — a `.lua` / `.st` file was successfully parsed (with
    a depot count — **not** the IDs)
  - `download_started` / `download_completed` — with a success flag and
    a depot count
  - `shortcut_created` — a Windows shortcut was created
  - `update_checked` / `update_installed` — the auto-updater ran

## What is NEVER collected

- Steam App IDs, depot IDs, manifest IDs
- Contents of your `.lua` / `.st` files
- Game names, cover art, descriptions
- File paths, directory contents, download targets
- Your GitHub / Steam credentials, API keys, or any tokens
- Your IP address (stripped at the reverse proxy before anything reaches
  the server log)
- Any identifier linked to your operating-system user account

## Encryption

Events are batched in memory, serialized as JSON, and encrypted with a
`crypto_box` sealed box using an X25519 public key that ships inside the
binary. Only the private key on the maintainer's telemetry server can
decrypt the payload — not the hosting provider, not intermediate proxies,
not anyone observing the connection. TLS is used on top of that.

## Retention

Decrypted events are appended to a rotating daily `.jsonl` file on the
server. They are not joined with any other dataset. There is no user
database. If you want a copy of what the server has associated with your
install UUID, or want it deleted, email the address below — but note that
because events don't carry any identifier beyond the random UUID that
only exists in your local settings file, there is typically nothing to
match against unless you send the UUID yourself.

## Opting out later

Turn the toggle off in **Settings → Advanced Settings**. After that no
events are sent and no installation UUID is used. Already-transmitted
events can't be un-sent (they've left your machine), but they're
anonymous and will age out of the server logs per the retention policy.

## Questions

Open an issue with the `question` template or email
`mcbabel.sup@protonmail.com`.