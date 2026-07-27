# ChemSema HarmonyOS PC

This is the first-stage HarmonyOS PC shell for ChemSema. It packages the existing web viewer and Rust/WASM engine into an ArkWeb-based HarmonyOS app, scoped to the `2in1` device type for desktop-class HarmonyOS devices.

## Commands

From the repository root:

```powershell
npm run harmony:sync-viewer
npm run harmony:build
```

`harmony:sync-viewer` copies `viewer/` into `entry/src/main/resources/rawfile/chemsema/`. The copied files are generated app assets and are intentionally ignored by Git.

The project-level `build-profile.json5` is local-only because DevEco Studio writes
debug signing certificate paths and passwords into it. The checked-in
`entry/build-profile.json5` contains only the module build definition and must remain
versioned. The command-line wrapper uses `build-profile.example.json5` whenever local
signing material is unavailable. Use DevEco Studio automatic signing to regenerate a
signed local project profile when installing to a device or emulator.

## Scope

- PC/2in1 only.
- No tablet-specific interaction work in this stage.
- No Office/OLE integration in this stage.
- Web UI plus WASM core, loaded locally through ArkWeb rawfile resources.
