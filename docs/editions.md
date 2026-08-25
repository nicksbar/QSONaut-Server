# Public and hosted editions

QSONaut Server is designed as a usable public/local foundation plus an
optional proprietary hosted extension.

The public edition supports personal operation, profiles, logs and internal
visibility,
station synchronization, contest/activity setup, and basic shared clubs. Club
support includes discovery, creation, basic club identity editing, activity
setup, member rosters, membership records, join-request review, and access
approval. The public edition does not expose officer or board assignment,
elections, voting cycles, payments, external share links, public profile/club
landing pages, or commercial organization controls. Internal operator,
station-link, club, and activity visibility remains available to authenticated
users. It allows at most five clubs per database. The limit is enforced in a transaction
with a PostgreSQL advisory lock so concurrent requests cannot create a sixth
club accidentally.

The private companion repository extends the same runtime for hosted
organizational governance and commercial services. It supplies a
`ServerPolicy` through `router_with_store_and_policy`; clients can discover the
active edition at `/api/v1/capabilities`.

The public server must never require OAuth, payment accounts, email delivery,
hosted CAPTCHA, or provider credentials. Those remain optional extensions so a
local or field deployment stays fully usable offline and without a hosted
commercial dependency.

The public repository contains no governance API, persistence types, or UI for
board management, officer assignments, elections, or voting cycles. Historical
migrations are immutable database history; new hosted deployments apply the
governance schema from the private companion.
