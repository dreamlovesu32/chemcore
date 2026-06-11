# Manual CDXML Fixtures

This directory contains CDXML files that were previously kept on the local
desktop or in the ignored `tmp/` workspace while debugging import, rendering,
toolbar, arrow, shape, orbital, and large-selection behavior.

Keep regression inputs here once they become important enough to reproduce a
bug or verify a fix. Files in this directory are tracked by git; avoid relying
on ignored `tmp/` files or local desktop copies for future test work.

- `desktop/`: hand-authored or manually exported files that had been kept on
  the Windows desktop during debugging.
- `tmp-top-level/`: top-level `tmp/*.cdxml` inputs that were unique and useful
  enough to preserve. Duplicate `tmp` files were removed after matching hashes
  against existing fixtures.
