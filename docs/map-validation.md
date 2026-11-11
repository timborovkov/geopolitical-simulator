# Global map validation evidence

This checkpoint concerns the native desktop presentation on
`feat/map-visual-overhaul`. It does not complete the game roadmap or certify a
hardware performance baseline. The approved contract is in
[map presentation](map-presentation.md).

## Assets and simulation separation

- The packaged atlas is approximately 80.3 MB: 258 country/territory reference
  features, 7,325 settlements, 682 terrain tiles and three country mesh levels.
  Source archives are excluded from the runtime package. The retained embedded
  fallback/administrative/hydrology JSON adds approximately 7.2 MB; combined map
  source assets total approximately 87.5 MB, below the 250 MB cap.
- `scripts/build_map_atlas.py --source-dir SOURCE --check` completed an isolated
  rebuild whose manifest and every output checksum matched the committed assets.
  The source lock rejects changed inputs before compilation.
- `python3 scripts/test_map_atlas.py` passes three tests covering global identities,
  all pyramid levels, missing/corrupt/unsafe assets, invalid coordinates and mesh
  indices. Triangle areas match the exterior-minus-holes areas of all 12,822
  polygons within 0.00001 square degrees; 39 polygons retain holes across the
  three levels. These checks include the actual source geometry, not just fixtures.
  Twenty decoded JPEG diagonal tile junctions have mean absolute RGB error 1.3
  and maximum channel error 6 (on the 0–255 scale), including longitude wrap.
- Rust checks cover geographic picking at all three detail levels, Italy outside
  the scenario, Fiji across the date line, London settlement identity, malformed
  tile manifests, adjacent tile edges, pointer-anchored date-line zoom, wrapped
  visibility, reference navigation history and actual mode/layer controls.
- Presentation tests retain exact campaign bytes at 100%, 140% and 200% scaling.
  Actual native saves after navigation, reference inspection, fallback loading and
  layer changes retain the opening campaign SHA-256
  `bfd2e7b4739100c657c65f7c9ff96bc0dcebf31cc89b5f37517d426d85f62ce8`.
  No simulation source, scenario pack, content hash contract or save format changed.

## Native Linux coverage

The release executable was exercised on Linux with an Xorg dummy display and
an AMD EPYC 9V74 host (four-CPU container quota) and Mesa llvmpipe
(LLVM 19.1.7, 256 bits), using wgpu's OpenGL backend. This is a
software GPU. Window input was sent through XTest; screenshots came from the
actual native window. Temporary campaign/settings directories kept these checks
separate from user files. Owned test clients and the test display were closed
after verification.

| Native case | Observed result |
|---|---|
| Full zoom sweep, 1280×800 / 100% | World view plus regional and close views of Europe, East Asia, North America and Fiji/date line: stable overhead camera, progressively revealed settlements, continuous terrain and readable controls. |
| Europe, 1920×1080 / 100% | Terrain, rivers, country outlines, city labels and Paris inspector; reference infrastructure switches independently. |
| Rome outside the scenario | Italy/Rome shows reference-only attribution, without government actions or invented statistics. |
| East Asia, 1280×800 / 100% | Japan, Korea and eastern China retain terrain, capital/city labels and a bounded inspector. |
| Fiji/Pacific, 1920×1080 / 100% | Terrain and islands continue across the date line; Fiji remains reference-only. |
| World and North America, 3840×2160 / 200% | Scaled controls, terrain, hydrology, cities and country inspection remain visible. |
| Search and navigation | Scenario search and global place search focus countries; city picking opens its inspector; analytical-screen return and Back/Forward preserve presentation. |
| Missing atlas, 1440×940 / 140% | Campaign loads with the simplified map and a visible missing-assets diagnostic. |
| Missing base tiles, 1440×940 / 140% | A valid manifest and vector atlas without the two base images selects the existing simplified map; campaign save bytes remain unchanged. |
| Missing fine tiles, 1440×940 / 100% | A package containing only the two coarsest images still loads; missing detail is diagnosed and coarse imagery remains available. |

Additional offline GEOS checks found coincident shared borders and zero overlap
for France/Germany, Belgium/Netherlands, Italy/Austria, USA/Canada, China/Mongolia
and South Africa/Lesotho at all three detail levels (18 pair/level checks).

The [Europe screenshot](images/map-europe.png) records the actual native map.
Native inspection exposed and resolved the default GPU storage-buffer limit,
resource loss during new-campaign reset and missing horizontal map wrapping.
The final interaction tests additionally cover reference-only history entries.
Final asset review corrected diagonal gutter samples, preserved LF/POSIX compiler
output on other operating systems, and made absent base tiles choose the simplified
map. Terrain detail uses physical pixel density at scaled resolutions. The atlas
was rebuilt, reproduced, checked and repackaged after those corrections.
macOS and Windows native behavior and hosted CI have **not** been confirmed for
this branch.

## Performance evidence and limits

The opt-in `STATECRAFT_MAP_BENCHMARK=1` route uses 60 warmup frames followed by
360 measured frame intervals over Europe, Asia, America and the Pacific. It logs
adapter identity and tile-cache occupancy. These are whole frame intervals,
including UI work and presentation, not isolated GPU timestamps.

At 1440×940 / 100%, `LP_NUM_THREADS=2`, the Linux software renderer recorded
p50 **136.39 ms**, p95 **183.16 ms**, maximum **234.01 ms**. Four simulation replay processes
were running concurrently within the container's four-CPU allocation. This run
verifies rendering and bounded cache operation; it does not meet or establish the
60 FPS hardware target. A dedicated hardware run remains necessary.

The measured cache reached 88 tiles, with an image-storage upper bound of
123,995,989 bytes (118.25 MiB), below the 128 MiB limit. Driver overhead and static
mesh buffers are separate from that image budget. The four-result decode queue,
worker and upload buffer bound decoded image storage below 10 MiB, within the
64 MiB cap; this is a structural bound, not a process RSS measurement. Source
atlas structures and application memory are separate costs.

## Repository validation

All **170 commands** from `python3 scripts/validate.py` completed successfully:
727 unique Rust checks (124 desktop and 70 compile-fail contracts), 13 Python
checks, strict lints, Rust documentation, release builds, `headless --verify` and
all 1,296 recovery/replay campaigns. See the
[machine-readable record](map-validation-results.json) for exact command arguments,
exit results, replay hashes, final source hashes, package sizes and benchmark data.

The 27-command serial prefix completed through the workspace test command. Its
725 checks include the earlier 122-test desktop snapshot; the final 124-test
desktop run supplies the two additional checks without counting repeats as new
tests. The 143 remaining commands retained their original argument lists. An
environment restart interrupted active commands; completed exit-0 results were
retained against the frozen source hashes and only unfinished commands were
restarted, with four workers. Interrupted partial logs are excluded from success
evidence. No uninterrupted full-validator elapsed time is claimed.

Simulation and original scenario inputs remained unchanged. Later presentation
and asset corrections received final desktop tests, strict workspace lints,
documentation/release builds, exact asset regeneration, integrity/packaging checks
and native inspection. The final source/asset snapshot and exercised native binary
were checked again after the gate.

Release acceptance remains open for dedicated hardware performance measurements
and macOS/Windows validation. Street-level detail,
individual buildings, dynamic terrain and simulated transport remain outside this
visual overhaul.
