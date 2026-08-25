# Public and hosted editions

QSONaut Server is designed as a usable public/local foundation plus an
optional proprietary hosted extension.

The public edition supports personal operation, profiles, logs and sharing,
station synchronization, contest/activity setup, and basic shared clubs. It
allows at most five clubs per database. The limit is enforced in a transaction
with a PostgreSQL advisory lock so concurrent requests cannot create a sixth
club accidentally.

The private companion repository extends the same runtime for hosted
organizational management and commercial services. It supplies a
`ServerPolicy` through `router_with_store_and_policy`; clients can discover the
active edition at `/api/v1/capabilities`.

The public server must never require OAuth, payment accounts, email delivery,
hosted CAPTCHA, or provider credentials. Those remain optional extensions so a
local or field deployment stays fully usable offline and without a hosted
commercial dependency.
