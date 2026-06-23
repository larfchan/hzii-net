# Security policy

## Credential handling

The campus Portal is served over HTTP. Its JavaScript applies AES-128-CBC with a fixed, publicly downloadable key and appends the IV to the ciphertext. Anyone who captures a complete login request can recover or replay the credential. This is a limitation of the upstream Portal and cannot be repaired by a compatible client.

- Never attach HAR files, real `config.toml` files, passwords, or session state to public issues.
- If a HAR or login request has been disclosed, change the campus-network password.
- Keep the configuration and session state readable only by the service account.
- The program never intentionally logs passwords, encrypted credentials, or session secrets.

## Reporting a vulnerability

Open a private GitHub security advisory instead of a public issue. Include a minimal reproduction with fake credentials and redact all request bodies and tokens.

