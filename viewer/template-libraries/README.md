# Local template libraries

This directory is populated from a locally installed, licensed copy of
ChemDraw:

```powershell
npm run generate:chemdraw-template-libraries
```

The generated `catalog.json` and `.cdxml` files are intentionally ignored.
They remain available to the local viewer and desktop application, but are not
redistributed under ChemSema's Apache-2.0 license.

After generation, validate every extracted template with:

```powershell
npm run gate:template-libraries
```

The gate parses, renders, serializes, exports, and checks semantic object
conservation for the complete local catalog.
