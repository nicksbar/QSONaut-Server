# Opt-in diagnostic bundles

QSONaut Server may accept diagnostic bundles from QSONaut clients to help reproduce radio integration problems and improve hardware support. This is a deliberate support workflow, not background telemetry.

## Consent and operator control

- Collection is disabled by default.
- Every collection run requires an explicit operator action and a plain-language acknowledgement of what will be gathered.
- QSONaut builds the bundle locally and shows a manifest and redacted preview before upload.
- The operator can save the bundle locally without uploading it, remove individual optional sections, or cancel.
- Consent applies only to that bundle. It does not silently enable future collection.

## Default bundle contents

- QSONaut version, operating-system family, and architecture
- Radio manufacturer/model selection and capability-probe results
- Driver and protocol traces with bounded timestamps and severity
- Audio and serial device capability summaries using per-bundle pseudonymous identifiers
- Test-step outcomes, timeouts, and sanitized error chains
- A random bundle identifier and the operator-selected issue category

The default bundle must not include raw audio, decoded message contents, QSO/logbook records, callsigns, operator names, email addresses, stable hardware serial numbers, IP addresses, filesystem paths, authentication material, or server/session cookies.

## Redaction and transport

The client applies structured field allowlists and text redaction before presenting the preview. The server repeats validation and redaction, rejects secrets and unsupported files, enforces compressed and expanded size limits, and records the applicable schema/redaction version. Uploads use authenticated HTTPS and encryption at rest. Download access is limited to authorized maintainers and audited.

Useful network metadata that cannot be avoided during transport, such as the source IP visible to the receiving proxy, is not copied into the diagnostic record and follows a short infrastructure-log retention policy.

## Retention and analysis

- Bundles have a visible retention deadline and are deleted automatically.
- The submitter can delete a bundle early using a one-time receipt or authenticated account.
- Reports and update decisions use aggregate findings where possible.
- A bundle is never used for advertising, operator tracking, or unrelated analytics.
- QSONaut Server cannot use diagnostics to control a radio, PTT, audio, or modem activity.

## Proposed API boundary

The future protocol should use a versioned manifest, a short-lived upload grant, and a separate immutable bundle object. Collection policy and schemas belong in the shared protocol crate; radio-specific probes remain in QSONaut. No ingestion endpoint should ship until redaction fixtures, malicious-archive tests, authorization tests, retention deletion, and a complete client preview workflow exist.
