# ML Studio supporter service

This reuses the existing Ko-fi/personal-PayPal shop and the VoicePilot signed-activation design. PHP sodium and the app's existing `ed25519-dalek` verifier provide Ed25519 signatures. No new app runtime, payment SDK, subscription, microphone access, machine fingerprint or recurring online check is required. The signing authority and product are separate from VoicePilot.

- Shop item: https://ko-fi.com/s/833f0d3495 (minimum EUR 10, Pay what you want enabled).
- Delivery: https://michaellanemusic.de/ml-studio-supporter/ . Ko-fi's buyer message/receipt links here. This service does **not** send a separate email.
- Existing notification endpoint remains `https://michaellanemusic.de/voicepilot/api.php?action=kofi-webhook`. A narrow router handles only this product and otherwise falls through to the existing VoicePilot handler.
- Public files: `Supporter.php`, `index.php`. Never deploy `test.php` permanently. There is no public administrative issuance endpoint.
- Private storage: `/home/users/michaellane-music/files/ml-studio-supporter`, outside the website. `authority.seed` is the persistent signing authority; `supporters.sqlite` stores one activation per verified transaction. Do not delete/recreate either on deployment. Back them up through the host's private backup/recovery process. Never put them in Git, a public download, test output or the portable source archive. If the authority is lost while the database exists, startup fails closed instead of replacing it.

The existing VoicePilot private configuration supplies **only** the Ko-fi verification token. It is not copied into this source or into the app. The app contains only the distinct public verification key. A key change would invalidate existing activations; normal updates must retain it.

## Fulfilment and recovery

Ko-fi reports completed payments. Accept a single supporter item, quantity one, in EUR, at least EUR 10. Reject invalid authentication, underpayment, subscriptions and ambiguous mixed baskets. Ko-fi reports the whole order total, so a mixed basket or discounted order cannot safely prove this item's paid minimum; tell buyers to use a separate checkout and review exceptions manually. Preserve the €5 VoicePilot product's existing fulfilment rules.

After verification, buyers enter their purchase email and **Ko-fi transaction ID** on the delivery page. A shared delivery URL alone cannot obtain a licence. The lookup uses keyed hashes; raw email and transaction IDs are not stored in the ML database. Rate limits expire after an hour; raw IP addresses are not retained by this application. Host access logging remains a hosting setting. Duplicate notifications retain the same activation. Signed files contain a random licence ID and product entitlement, without buyer contact data. Ko-fi does not send refund/revocation events through this integration; offline perpetual certificates do not support remote revocation.

The buyer imports `.mlsupport` through Donate > Activate supporter or Settings > General > Support ML Studio. The app verifies before an atomic save to the existing profile's `Licenses` directory. A bad import leaves an existing activation untouched. Reinstalls/new profiles can import the saved file. Projects, theme resets, installer replacements and ordinary updates do not own or remove it. This is a personal portable certificate, not copy protection against deliberate file sharing or modified GPL builds.

## Validation and release

Run `php test.php` in an isolated environment with sodium and PDO SQLite. It covers authentication, price/currency, mixed orders, idempotency, signatures, recovery, higher contributions and throttling. The optional Rust `ML_SUPPORTER_SERVER_FIXTURE` test accepts that test certificate only against its test public key; production verification has no test bypass.

The app tests exercise real toolbar/settings entry points, default and vintage skins, toolbar editing and hiding, legacy visibility preferences, file import persistence, failed imports, successful/failed audio export and video completion/cancellation. Build with the normal public configuration, then check clean-profile startup and the installer/updater preservation of the licence directory. The public installer and updater must never include a buyer's activation.

This shop item must remain Draft until a public app containing activation is published. The user explicitly corrected the earlier choice to open it for Development. Publish the prepared item and remove the delivery page’s pending-launch notice only as part of that public release. A real Ko-fi purchase, its receipt delivery, and successful import in the installed app still require an explicitly authorized payment test; do not report a simulated notification as a paid checkout. Do not charge the owner or email a test customer automatically.

The fixed Donate button intentionally has no Workspace Visibility switch; activation removes it. Hiding the rest of the toolbar leaves a compact fixed supporter button. The reminder is dismissible and never blocks access to the exported file or audio features. It appears only after successful final user exports, not internal renders, failures or cancellations.
