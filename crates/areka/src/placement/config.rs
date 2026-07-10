//! descript KV → `PlacementConfig` 変換（task 3 で実装）。
//!
//! 4 層カスケード・両表記寛容（`defaultx`/`defaultleft` 等）・scope 検出・
//! zorder/sticky/dpi 転記シーム。入力は BTreeMap×2（純粋）。
