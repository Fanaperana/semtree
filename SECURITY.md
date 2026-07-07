# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

If you discover a security vulnerability in SemTree, please report it responsibly:

1. **Do not** open a public GitHub issue for security vulnerabilities
2. Email: **[INSERT EMAIL]** or use GitHub's private vulnerability reporting
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

## Response Timeline

- **Acknowledgment**: Within 48 hours
- **Initial assessment**: Within 1 week
- **Fix and release**: As soon as practical, typically within 2 weeks

## Scope

This security policy covers:
- The `semtree` CLI binary
- All `semtree_*` library crates
- The C FFI API (`semtree_ffi`)
- WASM bindings
- Grammar parsing (malicious grammar files)

## Known Security Considerations

- **Grammar parsing**: SemTree parses user-supplied grammar files. The parser includes depth guards and iteration limits to prevent stack overflow and infinite loops from malicious inputs.
- **FFI boundary**: The C FFI uses opaque pointers with null-safety checks. Invalid pointers will not cause undefined behavior (functions return null/zero).
- **No network access**: SemTree does not make network requests. All operations are local.
