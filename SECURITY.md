# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

If you discover a security vulnerability in Conative Gating, please report it responsibly:

1. **Email**: security@hyperpolymath.org
2. **Subject**: `[SECURITY] conative-gating: Brief description`
3. **Include**:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact assessment
   - Any suggested fixes (optional)

### Response Timeline

- **Initial acknowledgment**: Within 48 hours
- **Triage and assessment**: Within 7 days
- **Fix or mitigation**: Depends on severity
  - Critical: Within 7 days
  - High: Within 30 days
  - Medium/Low: Next release cycle

### What to Expect

- We will acknowledge receipt of your report
- We will investigate and keep you informed of progress
- We will credit you in the security advisory (unless you prefer anonymity)
- We will not take legal action against good-faith security researchers

## Security Considerations

### Policy Oracle

The Policy Oracle performs deterministic rule checking:
- File extension and content marker detection
- Pattern matching for forbidden content (secrets, banned languages)
- No external network calls during evaluation

### SLM Evaluator (Planned)

Future SLM integration will:
- Run locally using llama.cpp (no external API calls)
- Use quantized models for reduced attack surface
- Implement input sanitization before inference

### Consensus Arbiter (Planned)

The Elixir arbiter will:
- Use supervision trees for fault tolerance
- Implement rate limiting to prevent DoS
- Log all decisions for audit purposes

## Hardening Recommendations

When deploying Conative Gating:

1. Run with minimal privileges
2. Use read-only access to scanned directories where possible
3. Validate all external inputs (proposal JSON schemas)
4. Review audit logs regularly
