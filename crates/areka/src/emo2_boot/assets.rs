//! 構築入力（BootAssets）の組立と shell descript からの static bindset 抽出。
//!
//! `build_boot_assets`（shell: `surfaces.txt` 読取→`areka_parsers::shell::parse`→bake→scope ごとに
//! `EmoWorld::build`＋`bind_atlas`／balloon: `build_balloon_target`＋`BalloonModel`／
//! `SurfaceResolver`＝`alias_snapshot()`／static bindset＝`default_bind_ids`→`build_static_bindset`）と
//! `default_bind_ids`（`sakura.bindgroup{N}.default==1` の N 抽出・DD-8・ukadoc 正典）を所有する。
//! 戻り値だけで以後ファイル I/O 不要にする（parse／bake は 1 回・`AtlasTable` は Clone 共有）。
//! 失敗は `BootWiringError`（`#[from]` 変換群）で観測可能化する。
//!
//! 骨格のみ。実装は tasks.md task 2.3（`default_bind_ids`）／task 2.6（`build_boot_assets`）が担う。
