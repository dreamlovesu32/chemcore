# Security Policy

ChemCore is not yet a mature security-hardened application. Please treat file
import, Office/OLE integration, and clipboard handling as security-sensitive
areas.

## Reporting a Vulnerability

Do not post exploit details publicly. Send security reports to
dreamlovesu@hotmail.com with a concise subject such as
`ChemCore security report`.

Useful private reports should include:

- affected commit or release
- operating system and application surface, such as browser viewer, desktop, or
  Office/OLE
- minimal reproduction file when it is safe to share
- expected impact
- whether the issue is already known to third parties

## Scope

Security-sensitive areas include:

- CDXML/CDX/SDF and native document parsing
- Office/OLE object activation and clipboard formats
- filesystem access in the desktop shell
- generated previews and export paths
- denial-of-service inputs that hang or exhaust memory

Please avoid testing against systems or documents you do not own or have
permission to analyze.
