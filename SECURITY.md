# Security Policy

## Supported versions

| Version | Supported |
|---|---|
| 0.1.x | current development, mitigated as fixes land |

## Reporting a vulnerability

Keep security problems out of public issues.

- Use GitHub private vulnerability reporting for this repository.
- Alternatively email the maintainers directly (address found in commit metadata).

Provide:

- affected version
- description of the issue
- steps to reproduce
- impact assessment

## What we commit to

- acknowledge reports within 5 business days
- keep you updated on the fix timeline
- never log or expose provider API keys
- never commit secrets
- treat redaction of secrets in output/`config show` as a bug if broken

## Security baseline (v0.1)

- server binds to localhost by default
- no permissive CORS by default
- Docker runs non-root
- backup/restore filenames sanitized, path traversal rejected
- imported backups validated before restore