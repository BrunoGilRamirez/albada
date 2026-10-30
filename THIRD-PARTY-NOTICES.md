# Third-Party Notices — Albada

> Status: staged, owner-review-pending notices and inventory for the public
> `albada` repository (2026-10-29). This supersedes the preliminary
> inventory-only pass of the same date. It combines the automated
> license-metadata scans with the upstream legal texts retained verbatim in
> this repository. It is **not** a legal sign-off and **not** a compatibility
> determination; the scan sections are metadata inventories and the final
> scan re-run by L0 is pending. Remaining open items are listed in section 7;
> **owner approval of the font/asset notices and of the MPL-2.0 notices
> remains OPEN**. Owner/L3 review and the final verification re-runs (L0)
> must complete before publication (Phase 10.0/10.1/10.2/10.3 checkboxes).

## Scope and sources

- Application staged tree: `public-repo/` (future public `albada` repository
  root). Rust backend: `src-tauri/`; frontend: npm/TypeScript.
- Lockfiles at scan time (dependency graphs unchanged by the scans and by
  the notices passes; the `package-lock.json` root entry additionally
  carries the synced `license` metadata — root metadata only, no dependency
  versions changed):
  - `public-repo/src-tauri/Cargo.lock`
  - `public-repo/package-lock.json`
- Scan commands (metadata inventory only; re-run by the final verification
  pass after this staging):
  - Rust:
    `cargo-license.exe --manifest-path src-tauri/Cargo.toml --json`
    against `src-tauri/Cargo.lock`.
  - npm: `npm ci` first (installed `node_modules` only; lockfile unchanged),
    then
    `npm exec --yes --package=license-checker-rseidelsohn -- license-checker-rseidelsohn --production --json --start .`
    against `public-repo/package.json` / `public-repo/package-lock.json`.
- Embedded Typst assets: `typst-assets` pinned at 0.15.1 in
  `public-repo/src-tauri/Cargo.lock`, used **without** its `fonts` feature
  (the Rust `World` font loader instead uses an explicit six-file allowlist
  of staged Libertinus faces; see section 2). Inventory sources are the
  exact crates.io `typst-assets` 0.15.1 asset list and the upstream files
  `typst-assets-0.15.1/src/lib.rs`, `LICENSE`, and `NOTICE` from
  `https://github.com/typst/typst-assets/tree/v0.15.1`. The upstream legal
  files are now retained verbatim in this tree (section 3).

## 1. Application license

- Albada (root package) — Copyright © 2026 Bruno Gil Ramírez.
- Source license: **`AGPL-3.0-or-later`** (owner decision recorded
  2026-10-29).
- Root [`LICENSE`](./LICENSE) holds the verbatim official GNU AGPL v3 text
  fetched from `https://www.gnu.org/licenses/agpl-3.0.txt`; no header was
  added and the text is unmodified.
- SPDX metadata added in these passes: the `license` field in `package.json`
  and the `license` key in `src-tauri/Cargo.toml` are set to
  `AGPL-3.0-or-later`, and the root entry of `package-lock.json` carries the
  same `license` value (root metadata sync only; no dependency versions
  changed). With this metadata the root package reports `AGPL-3.0-or-later`;
  the external crate and dependency counts in sections 4 and 5 are unchanged.
- No legal terms are invented in this tree. The root
  [`NOTICE`](./NOTICE) is a short factual project notice (copyright holder,
  `AGPL-3.0-or-later` reference, the Aurora icon declaration, and paths to
  the third-party notices and retained upstream license files); it adds no
  license conditions. The upstream Typst legal files and the verbatim
  Unicode-3.0 and MPL-2.0 license texts are preserved under `third-party/`
  (sections 3, 4, and 6).

## 2. Fonts distributed with Albada

The Rust `World` font allowlist stages and loads **exactly six font
faces** from `src-tauri/assets/fonts/` (this is not the upstream
`typst-assets` full font inventory; the Foxit/PDFium record below states
exactly what Albada does and does not claim):

- `LibertinusSerif-Regular.otf`
- `LibertinusSerif-Bold.otf`
- `LibertinusSerif-Italic.otf`
- `LibertinusSerif-BoldItalic.otf`
- `LibertinusSerif-Semibold.otf`
- `LibertinusSerif-SemiboldItalic.otf`

License: **SIL Open Font License 1.1** — Copyright © 2012–2024 The
Libertinus Project Authors, with Reserved Font Names "Linux Libertine",
"Biolinum", "STIX Fonts" (attribution per the pinned upstream Typst
NOTICE). The full OFL-1.1 text is reproduced verbatim inside
`third-party/typst-assets/NOTICE` and is not re-drafted here.

Provenance: the six staged files were byte-verified against the exact
upstream `typst-assets` v0.15.1 Git blob hashes, and Rust tests prove the
font loader exposes these six Libertinus faces only. The embedded default
Typst export template (version 1.0.1) explicitly sets `font: "Libertinus
Serif"`, which resolves to the bundled OFL faces.

Excluded from the `World` font allowlist: the other text-layout font files
in the upstream `typst-assets` inventory — the New Computer Modern (NewCM)
family under the GUST Font License 1.0 (LPPL-related terms recorded
upstream), `NewCM10-Regular.otf` under GPL-3.0-or-later with
font/distribution exceptions (excluded by owner decision; no compatibility
claim is made for it), and the DejaVu fonts under Bitstream Vera–based
terms — are **not** staged as Albada font assets and are **not** loaded by
the Rust `World`. Their upstream terms are preserved verbatim in
`third-party/typst-assets/NOTICE` for completeness of the Typst record.

Foxit/PDFium base-14 record (claim withdrawn and corrected): an earlier
pass described the Typst PDF base-14 Foxit/PDFium fonts as bundled
through the PDF module. That claim is withdrawn. The L0 source search of
the pinned consumer crates used by Albada — `typst-pdf-0.15.1`,
`typst-library-0.15.1`, and `typst-html-0.15.1` — found no
`typst_assets::pdf` references; the Foxit constants are declared only in
the upstream `typst-assets` crate itself. Albada therefore makes **no**
claim that the Foxit/PDFium base-14 fonts are bundled with or used by
Albada. The retained upstream `third-party/typst-assets/NOTICE` is
reproduced byte-verbatim and includes its Foxit/PDFium ("Copyright 2014
PDFium Authors") BSD-style terms as part of Typst's upstream asset
record; that retained upstream section is explicitly distinct from the
fonts and assets Albada bundles or uses, whose font allowlist is exactly
the six Libertinus OFL faces listed above.

Resolution of preliminary blocker #1 (`NewCM10-Regular.otf` Distribution
Exception vs. `AGPL-3.0-or-later`): resolved **by exclusion**. Because the
`typst-assets` `fonts` feature was removed, the font is not staged as an
Albada asset, and `NewCM10-Regular.otf` is excluded by owner decision, no
AGPL/GPLv3-compatibility determination is made or claimed for it in the
staged tree.

## 3. Embedded Typst compiler assets (`typst-assets` 0.15.1)

The `typst-assets` crate itself is Apache-2.0. The exact upstream legal
files are copied **verbatim and unaltered** into this tree:

- `third-party/typst-assets/LICENSE` — Apache License 2.0, from
  `https://raw.githubusercontent.com/typst/typst-assets/v0.15.1/LICENSE`
- `third-party/typst-assets/NOTICE` — the full upstream third-party asset
  notice (which contains the verbatim OFL-1.1, GUST, Bitstream Vera/DejaVu,
  Foxit/PDFium BSD-style, CC0-1.0, CC BY 4.0 + BSD-3-Clause, W3C, and
  GPL-3.0 texts for the assets in Typst's own upstream asset record — an
  upstream record, not a statement of which assets Albada bundles or uses;
  see section 2), from
  `https://raw.githubusercontent.com/typst/typst-assets/v0.15.1/NOTICE`

Additional verbatim upstream license texts retained in this tree for the
asset and crate attributions named above (referenced from the root
`NOTICE`):

- `third-party/icu4x/LICENSE` — Unicode License V3 (`Unicode-3.0`): the
  top-level `LICENSE` of the pinned `https://github.com/typst/icu4x`
  branch `cj-patch-update`, the data source of the embedded ICU blob
  (section 3.2).
- `third-party/unicode-ident/LICENSE-UNICODE` — Unicode License V3
  attribution text, copied from the official locked `unicode-ident`
  1.0.24 source at
  `https://raw.githubusercontent.com/dtolnay/unicode-ident/1.0.24/LICENSE-UNICODE`
  (section 4).
- `third-party/MPL-2.0.txt` — the official Mozilla Public License 2.0
  text, copied from
  `https://www.mozilla.org/media/MPL/2.0/index.txt` (section 4).

Asset-level attribution summary (this section supplements, and never
replaces, the verbatim upstream texts):

### 3.1 ICC profiles (`files/icc/`)

- `icc/sGrey-v4.icc` and `icc/sRGB-v4.icc` — **CC0-1.0**, explicitly covered
  by the upstream NOTICE entry "The Creative Commons Zero v1.0 Universal
  License applies to: … The ICC profiles found in `files/icc/*-v4.icc`".
- `icc/CMYK-to-XYZ.icc` — provenance resolved: the pinned Typst file is
  byte-identical to `saucecontrol/Compact-ICC-Profiles`
  `profiles/CGATS001Compat-v2-micro.icc` — both carry upstream Git blob SHA
  `b5a73495bfc7509837b300800d8197d94e83fab7` at 8464 bytes. The
  Compact-ICC-Profiles root `license` is **CC0-1.0**. The pinned Typst
  `src/lib.rs` source comment points to
  `https://github.com/saucecontrol/Compact-ICC-Profiles#cmyk` and to CGATS
  TR 001-1995. The preliminary "binary identity not asserted" blocker is
  closed by this exact blob match.

### 3.2 ICU data (`files/icu/`)

- `icu/icu_cj_segment.postcard` — asset-specific attribution recorded as
  required: the blob is generated by the custom `icu4x-datagen` workflow
  from `https://github.com/typst/icu4x` branch `cj-patch-update` (per the
  pinned Typst `src/lib.rs`), and that branch's top-level `LICENSE` is
  **Unicode-3.0**. The related locked ICU4X crates are listed among the 28
  `Unicode-3.0` crates in section 4. The exact license text of that branch
  is now retained verbatim at `third-party/icu4x/LICENSE`; the owner/asset
  review of the inclusion remains open (section 7).

### 3.3 HTML/Fetch standards data (`files/html/data.rs`)

- Data derived from the WHATWG HTML and Fetch standards
  (`https://html.spec.whatwg.org`, `https://fetch.spec.whatwg.org`),
  Copyright © WHATWG (Apple, Google, Mozilla, Microsoft).
- **Upstream documentation inconsistency — recorded, not corrected
  (informational; not a redistribution blocker).** The Typst NOTICE
  section heading reads "The Creative Commons Zero 4.0 International Public
  License as well as BSD 3-Clause apply to:" while the license text
  immediately following is Creative Commons **Attribution** 4.0 (CC BY 4.0),
  with BSD-3-Clause for source-code portions. That original upstream heading
  and text are preserved verbatim in `third-party/typst-assets/NOTICE`
  exactly as published; the copied NOTICE is not altered. Official
  cross-reference: the upstream WHATWG HTML and Fetch repository `LICENSE`
  files both identify CC-BY-4.0 plus BSD-3-Clause for the source portions.
  This heading/text mismatch is recorded here as an informational upstream
  documentation inconsistency, not an unresolved redistribution blocker;
  any upstream clarification can be picked up in a later pinned-version
  review.

### 3.4 W3C-derived data

- `files/html/data.rs` contains W3C-derived data from WAI-ARIA 1.1 and
  Referrer Policy (© 2017 W3C); `files/mathml/data.rs` contains MathML Core
  data (© 2025 W3C).
- These are under the W3C Document License with the exact
  sources/attributions/conditions stated in the upstream NOTICE. That
  content is preserved verbatim in the retained NOTICE copy and is not
  re-authored here.

## 4. Rust crate license-metadata inventory (cargo-license)

Scan output preserved from the preliminary pass as the metadata record; the
verification pass re-runs the scan.

- 688 records total, including the `albada` root package; **687 third-party
  crates**.
- No third-party crate had missing license metadata.
- No `GPL-2.0-only` identifier and no proprietary license identifier was
  found in the third-party set.
- The only LGPL identifier observed is `LGPL-2.1-or-later`, appearing as an
  **OR option** alongside Apache-2.0/MIT (permissive option available).
- These are SPDX metadata observations only; they are not a legal approval
  and not a compatibility analysis.
- After the license-metadata addition described in section 1, the root
  package is `AGPL-3.0-or-later`; the 687 external crate counts do not
  change.

### Third-party SPDX license-expression counts (scanner metadata)

Explicitly grouped as cargo-license scanner metadata output — **not** notices
and not legal conclusions. The counts cover all 687 third-party crates
(verified sum), recorded here so the full scan result is captured by
expression. Cross-checks against the package tables below: `MPL-2.0` = 5
rows-matched crates; `Unicode-3.0` (27) + `(Apache-2.0 OR MIT) AND
Unicode-3.0` (1) = 28 crates.

| License expression (as scanned)                       | Count |
| ----------------------------------------------------- | ----- |
| `Apache-2.0 OR MIT`                                   | 410   |
| `MIT`                                                 | 149   |
| `Unicode-3.0`                                         | 27    |
| `Apache-2.0 OR MIT OR Zlib`                           | 24    |
| `Apache-2.0`                                          | 21    |
| `MIT OR Unlicense`                                    | 14    |
| `Apache-2.0 OR Apache-2.0 WITH LLVM-exception OR MIT` | 6     |
| `MPL-2.0`                                             | 5     |
| `BSD-3-Clause`                                        | 4     |
| `Zlib`                                                | 4     |
| `Apache-2.0 OR BSD-3-Clause`                          | 3     |
| `BSD-2-Clause`                                        | 3     |
| `Apache-2.0 OR BSD-2-Clause OR MIT`                   | 2     |
| `Apache-2.0 OR BSD-3-Clause OR MIT`                   | 2     |
| `Apache-2.0 OR LGPL-2.1-or-later OR MIT`              | 2     |
| `Apache-2.0 WITH LLVM-exception`                      | 2     |
| `(Apache-2.0 OR MIT) AND Unicode-3.0`                 | 1     |
| `0BSD OR Apache-2.0 OR MIT`                           | 1     |
| `0BSD OR CC0-1.0`                                     | 1     |
| `Apache-2.0 AND MIT`                                  | 1     |
| `Apache-2.0 OR BSL-1.0`                               | 1     |
| `Apache-2.0 OR CC0-1.0 OR MIT-0`                      | 1     |
| `BSD-3-Clause AND MIT`                                | 1     |
| `BSD-3-Clause OR MIT`                                 | 1     |
| `ISC`                                                 | 1     |

### MPL-2.0 crates (complete list of the five locked crates)

| Crate              | Version |
| ------------------ | ------- |
| `cssparser`        | 0.37.0  |
| `cssparser-macros` | 0.7.1   |
| `selectors`        | 0.38.0  |
| `dtoa-short`       | 0.3.5   |
| `option-ext`       | 0.2.0   |

MPL review note (inventory only, not legal sign-off): the official Mozilla
MPL 2.0 — text retained verbatim at `third-party/MPL-2.0.txt`, copied from
`https://www.mozilla.org/media/MPL/2.0/index.txt` — defines AGPL v3.0 as a
Secondary License (Section 1.12) and permits larger works under terms of
choice subject to MPL obligations (Section 3.3). All five locked crate
source trees were searched and no attached "Incompatible With Secondary
Licenses" marker was found outside the license text/template. **Owner
approval of the MPL notice inclusion remains OPEN** (section 7).

### Crates carrying `Unicode-3.0` (complete list, 28 crates)

Source for the observed Unicode-3.0 attribution requirement: the upstream
`unicode-ident` `LICENSE-UNICODE` file / Unicode License V3, which states
that the copyright + permission notice must appear with copies of the data
or in associated documentation. The exact required legal text is now
included verbatim in this tree at
`third-party/unicode-ident/LICENSE-UNICODE` (official `unicode-ident`
1.0.24 source), with the branch-level copy at `third-party/icu4x/LICENSE`.
The asset-specific attribution for the embedded Typst ICU blob is recorded
in section 3.2 (typst/icu4x branch `cj-patch-update`, Unicode-3.0).
**Open item:** the attribution text is now included; owner/asset review of
the final inclusion remains pending (section 7).

| Crate                      | Version                                                             |
| -------------------------- | ------------------------------------------------------------------- |
| `icu_collator`             | 2.3.1                                                               |
| `icu_collator_data`        | 2.3.0                                                               |
| `icu_collections`          | 2.3.0                                                               |
| `icu_locale`               | 2.3.1                                                               |
| `icu_locale_core`          | 2.3.0                                                               |
| `icu_locale_data`          | 2.3.0                                                               |
| `icu_locale_fallback`      | 2.3.0                                                               |
| `icu_locale_fallback_data` | 2.3.0                                                               |
| `icu_normalizer`           | 2.3.0                                                               |
| `icu_normalizer_data`      | 2.3.0                                                               |
| `icu_properties`           | 2.3.0                                                               |
| `icu_properties_data`      | 2.3.0                                                               |
| `icu_provider`             | 2.3.1                                                               |
| `icu_provider_blob`        | 2.3.0                                                               |
| `icu_segmenter`            | 2.3.0                                                               |
| `icu_segmenter_data`       | 2.3.0                                                               |
| `litemap`                  | 0.8.3                                                               |
| `potential_utf`            | 0.1.6                                                               |
| `tinystr`                  | 0.8.4                                                               |
| `unicode-ident`            | 1.0.24 (observed expression: `(Apache-2.0 OR MIT) AND Unicode-3.0`) |
| `writeable`                | 0.6.4                                                               |
| `yoke`                     | 0.8.3                                                               |
| `yoke-derive`              | 0.8.2                                                               |
| `zerofrom`                 | 0.1.8                                                               |
| `zerofrom-derive`          | 0.1.7                                                               |
| `zerotrie`                 | 0.2.5                                                               |
| `zerovec`                  | 0.11.8                                                              |
| `zerovec-derive`           | 0.11.6                                                              |

## 5. npm production-dependency inventory (license-checker-rseidelsohn)

Updated scan after the direct production `zod` upgrade to `^4.6.6` (lockfile
resolves `zod@4.6.7`, MIT). Recorded as **metadata inventory only**; the L0
verification pass re-runs the scan as part of final verification.

- 121 scanner entries total, including the `albada` app root; **120
  third-party production dependencies** (including their production
  transitive closure).
- Observed license distribution (third-party entries): 113 MIT,
  1 Apache-2.0 OR MIT, 4 MIT OR Apache-2.0, 1 ISC, 1 Apache-2.0.
- No third-party missing-license or proprietary-license entry was reported.
- The earlier scan reported the app root as `UNLICENSED` because no SPDX
  metadata existed yet; the root `package.json` now carries
  `AGPL-3.0-or-later` (section 1). The root entry is the application itself
  and must not be treated as a third-party dependency.

## 6. Staged legal-file layout

- `LICENSE` — verbatim GNU AGPL-3.0 text for the Albada source (section 1).
- `NOTICE` — short factual project notice: copyright holder,
  `AGPL-3.0-or-later` reference, Aurora icon declaration, and paths to the
  third-party notices below; it adds no legal terms.
- `THIRD-PARTY-NOTICES.md` — this staged notices/inventory document.
- `third-party/typst-assets/LICENSE` — verbatim upstream Apache-2.0 license
  for `typst-assets` 0.15.1.
- `third-party/typst-assets/NOTICE` — verbatim upstream Typst third-party
  notice, including the full OFL-1.1 text and every embedded-asset
  license/notice section (also the Foxit/PDFium BSD-style section, which is
  part of the upstream record and asserts no use by Albada — section 2).
- `third-party/icu4x/LICENSE` — verbatim Unicode-3.0 license of the pinned
  typst/icu4x branch `cj-patch-update` (sections 3 and 3.2).
- `third-party/unicode-ident/LICENSE-UNICODE` — verbatim Unicode License V3
  attribution text from the locked `unicode-ident` 1.0.24 (sections 3 and 4).
- `third-party/MPL-2.0.txt` — verbatim official MPL-2.0 text (sections 3 and 4).
- `src-tauri/assets/fonts/` — the six bundled OFL-1.1 Libertinus faces
  (section 2).
- Packaging follow-up (Phase 10.4, open TODO): include `LICENSE`, `NOTICE`,
  this file, and the retained `third-party/` license files as Tauri
  installer resources when the workflow is implemented. No bundle-config
  change was made in this pass.

## 7. Open items before publication (owner / L3 / L0)

1. **Font/asset notice approval — OPEN:** owner review and approval of the
   font and asset notices (the six OFL-1.1 Libertinus faces — exactly the
   `World` font allowlist — and the retained Typst upstream NOTICE
   sections, including its Foxit/PDFium BSD-style terms, which belong to
   Typst's upstream asset record and are not fonts or assets claimed to be
   bundled or used by Albada (section 2); the
   ICC/ICU/HTML/Fetch/W3C asset attributions; and the root `NOTICE`
   wording) has not been given yet.
2. **MPL-2.0 notice approval — OPEN:** the five locked MPL crates (section 4) await explicit owner approval of their notice inclusion; the official
   license text is retained at `third-party/MPL-2.0.txt`.
3. **Unicode-3.0 attribution review — OPEN:** the attribution requirement,
   the 28-crate list (section 4), and the asset-specific record for the
   embedded ICU blob (section 3.2) are documented, and the exact Unicode
   License V3 texts are now included verbatim at
   `third-party/unicode-ident/LICENSE-UNICODE` and
   `third-party/icu4x/LICENSE`; owner/asset review of the inclusion still
   remains open.
4. **Verification re-runs:** the Rust/npm license scans are metadata
   inventories; the final L0 re-run (scans and production audit) is pending
   after this staging; the Phase 10.1/10.2 license-notice confirmations are
   re-checked by the owner after approval.

_Informational note (not an open blocker):_ the Typst NOTICE HTML/Fetch
heading/text inconsistency is recorded in section 3.3 as an upstream
documentation inconsistency. The copied `third-party/typst-assets/NOTICE`
is preserved verbatim and unaltered; the mismatch does not block
redistribution, and any upstream clarification can be picked up in a later
pinned-version review.

---

_Staged 2026-10-29 from the owner licensing decision and L0/L2 findings;
root `NOTICE` and the verbatim `third-party/` license copies (icu4x,
unicode-ident, MPL-2.0) added in this pass; the earlier Foxit/PDFium
bundling claim withdrawn after the L0 Typst-consumer-source search;
metadata scans are inventory, not legal conclusions; font/asset and MPL
notice approval remain OPEN pending owner/L3 review and the final L0
verification re-run before publication._
