# Security policy

## Reporting a vulnerability

Please do not open a public issue for a suspected security vulnerability.
Use GitHub's private advisory form:

<https://github.com/nicksbar/QSONaut-Server/security/advisories/new>

Include the affected commit or version, deployment type, reproduction steps,
and any relevant logs. Please redact passwords, device tokens, session cookies,
and database connection strings.

This project is maintained by volunteers. We will acknowledge reports as soon
as practical and coordinate a fix and disclosure timeline with the reporter.

## Deployment guidance

- Put the server behind HTTPS for any non-local deployment.
- Set `QSONAUT_SECURE_COOKIES=true` when using HTTPS.
- Use a unique, randomly generated PostgreSQL password.
- Never commit `.env`, device tokens, session cookies, or diagnostic payloads
  containing private station information.
