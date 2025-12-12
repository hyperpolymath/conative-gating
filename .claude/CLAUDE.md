# Project Instructions

## ⚠️ CONVERSION NEEDED: npm → Deno

This repo currently uses npm but needs to be converted to Deno.

### DO NOT:
- Run `npm install`
- Add new npm dependencies
- Create package-lock.json

### INSTEAD:
- Use `deno task` for scripts
- Use npm: specifiers for dependencies (e.g., `import X from "npm:package"`)
- Create deno.json instead of package.json

### Conversion Steps:
1. Analyze package.json dependencies
2. Create deno.json with equivalent imports/tasks
3. Update source files to use Deno imports
4. Remove package.json and package-lock.json
5. Test with `deno check` and `deno test`

## Package Manager Policy (RSR)

- **REQUIRED**: Deno for JavaScript/TypeScript
- **FORBIDDEN**: npm, npx, node_modules (after conversion)
- **FALLBACK**: Bun only if Deno is impossible

## Language & Security Policy (RSR)

### Allowed Languages (Primary → Fallback)
- **Systems/ML**: Rust
- **Web/Scripts**: ReScript → TypeScript (legacy only)
- **TUI**: Ada/SPARK
- **WordPress**: PHP (with security CI)
- **LSP**: Java (exception for IDE compatibility)

### Banned Languages
- Python (except SaltStack)
- Ruby (use Rust/Ada/Crystal)
- Perl (use Rust)
- New Java/Kotlin (except LSP)

### Package Management
- **Primary**: Guix (guix.scm)
- **Fallback**: Nix (flake.nix)

### Security Requirements
- No MD5/SHA1 for security (use SHA256+)
- HTTPS only (no HTTP URLs)
- No hardcoded secrets
