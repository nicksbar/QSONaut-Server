# Contest catalog

QSONaut Server owns its contest catalog. The Rust definitions in
`crates/qsonaut-contests` are the source of truth and are synchronized into
PostgreSQL after migrations run. Stable template UUIDs and contest keys are
part of the server contract; catalog revisions use `definition_version`.

The initial catalog was informed by the earlier Yahaml work, but no Yahaml
source, build artifact, database, or package is read at build time or runtime.
The data was normalized while moving it:

- modes use consistent `CW`, `PHONE`, `RTTY`, and `DIGITAL` terminology;
- event setup fields are distinct from fields captured for every QSO;
- recurring schedules are advisory rather than silently generating dates;
- each model links to the sponsoring organization's current rules;
- malformed labels and known stale contest details were corrected.

The catalog includes ARRL Field Day, Winter Field Day, POTA, SOTA, ARRL 10
Meter, both ARRL DX weekends, RTTY Roundup, 160 Meter, the January/June/
September VHF events, 10 GHz & Up, three Rookie Roundup modes, School Club
Roundup, a generic manual VHF model, and ARRL International Digital.

## Accuracy boundary

Templates guide event setup and describe expected bands, modes, exchange,
duplicate scope, and scoring shape. They are not yet a complete scoring or log
adjudication engine. The management UI displays the schedule as guidance and
requires an operator to choose exact event times. Operators should follow the
linked current rules when a sponsor changes an event.

Built-in rows are maintained by the server and refreshed on startup. A future
custom-template API should store user definitions separately and must not
overwrite reserved built-in contest keys.

## Planned activity geography

Contest definitions may eventually advertise optional geographic checkoff
dimensions such as Maidenhead grids, U.S. states, counties, or sponsor-defined
regions. These dimensions belong in the Activity workspace as opt-in overlays,
not as a permanently enabled global map feature.

The policy owner controls enablement for the relevant activity scope: an
operator for personal identities, and the organization or event owner for
club and event activity. Overlay progress must use only QSO records already
visible to the requesting operator, so enabling a checklist never broadens
log visibility. The community edition should provide basic bundled-boundary
checkoffs; richer region definitions, participant aggregation, validation,
progress history, and exports remain hosted capabilities.
