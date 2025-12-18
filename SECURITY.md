# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Security Measures

This project follows RSR (Rhodium Standard Repository) security practices:

- **SHA-pinned GitHub Actions**: All workflow actions use commit SHA pins for supply chain security
- **SPDX License Headers**: All source files contain SPDX license identifiers
- **OpenSSF Scorecard Compliance**: Continuous security monitoring via OSSF Scorecard
- **CodeQL Analysis**: Automated static analysis for security vulnerabilities
- **Dependency Scanning**: Automated alerts via Dependabot and cargo-audit
- **No Weak Cryptography**: SHA256+ required for security purposes (no MD5/SHA1)
- **HTTPS Only**: All external URLs must use HTTPS

## Reporting a Vulnerability

If you discover a security vulnerability, please report it responsibly:

1. **Do NOT** create a public GitHub issue for security vulnerabilities
2. Email the maintainer at: jonathan.jewell@gmail.com
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact assessment
   - Suggested fix (if any)

### Response Timeline

- **Initial Response**: Within 48 hours
- **Triage**: Within 7 days
- **Resolution**: Depending on severity
  - Critical: Within 7 days
  - High: Within 30 days
  - Medium/Low: Within 90 days

### Disclosure Policy

- We follow coordinated disclosure practices
- Credit will be given to reporters in the security advisory (unless anonymity is requested)
- Public disclosure after patch is released and users have reasonable time to update
