# 設計検証レポート: areka-P0-default-balloon-bundle

- 実施日: 2026-09-18
- 対象: `design.md`（2026-09-18 生成）・`requirements.md`（承認済み）・`research.md` §7〜§8
- 検証方法: 設計が依存する公開 API と番人（`spec_checks.rs` 腕 a〜f・`briefing_arms.rs`・`log-capture-kit`）を本ブランチのコードで実測して到達可能性を確かめた（非対話・報告のみ）。

## 設計レビュー要約

本番コード 0 行・新規依存 0 の「資産＋新規テスト 1 本＋文書」という最小形で、起動時と同じ公開 API 経路（`resolve_balloon_faces` → `build_balloon_target_from_faces` → `load_scope_balloon_model`・`ResolvedBalloonText::resolve`・`LayoutEngine::layout`）を `crates/areka-emo-text/tests/` から `Cargo.toml` 変更 0 で踏めることは実測で裏が取れた。檻 A〜I が呼ぶ公開 API・公開フィールドは**すべて実在し、綴りと引数順も設計どおり**である。残る欠陥は台帳側の 1 件——台帳の状態を `absent`→`degraded` へ変えると `briefing.md` の `[[barrier]]` 行が赤になるのに、設計の変更ファイル一覧に `briefing.md` が無い。これは 6.5 と 8.5 の両立を壊す到達可能な赤であり、設計ディスカッションで決めてから実装に入るべきである。

## 検証した論点（依頼の (a)〜(c)）

### (a) 新規テストが頼る公開 API の実在

| 設計の記述 | 実在の裏取り | 判定 |
|---|---|---|
| `areka_emo_present::balloon::{resolve_balloon_faces, load_scope_balloon_model, build_balloon_target_from_faces, ResolvedFace, ChainTier}` | `crates/areka-emo-present/src/balloon.rs` の `pub fn resolve_balloon_faces(&Path, u32) -> Result<Vec<ResolvedFace>, PresentError>`・`pub fn load_scope_balloon_model(&Path, u32, &ResolvedFace) -> BalloonModel`・`pub fn build_balloon_target_from_faces(&Path, &impl ElementDecoder, &[ResolvedFace]) -> Result<(EmoWorld, AtlasTable), PresentError>`。`ResolvedFace` の `surface_id`／`prefix`／`tier`／`file_name` は全部 `pub`。`lib.rs` が `pub mod balloon` | 一致 |
| `areka_emo_atlas::{WicDecoderArm, AtlasTable, SetId}`・`resolve`／`entry`／`page`・`original`／`placement`／`uv_rect`／`stride`／`bytes` | `decode/wic_arm.rs` の `pub fn new() -> windows::core::Result<Self>`・`table.rs` の `resolve(SetId, &str) -> Option<ElementId>`・`entry(ElementId) -> &AtlasEntry`・`page(u32) -> Option<&AtlasPage>`・各欄 `pub` | 一致 |
| `areka_parsers::{balloon::parse_str, charset::decode, kv::parse_kv, sakura::parse}` | `balloon/parse.rs` `parse_str(&str, Option<&str>) -> BalloonModel`・`charset/decode.rs` `decode(&[u8], DefaultEncoding) -> String`（`charset,Shift_JIS` の宣言を先読みして採用）・`kv/parse.rs` `parse_kv(&str) -> BTreeMap<String, String>`（CRLF 吸収・後勝ち）・`lib.rs` が 4 モジュールとも `pub mod` | 一致 |
| `BalloonModel` の `validrect()`／`font().name()`／`font().height()`／`font().color()`／`vertical_raw()`／`writing_mode()`／`wordwrappoint()` | `balloon/model.rs` に全部 `pub fn` | 一致 |
| `areka_emo_text::{actor::{ResolvedBalloonText, TextSlotBinding}, region::{TextRegion, ScaleContract, ImagePx}, draw::{ResolvedFont, DWriteMetrics, DEFAULT_FONT_NAME}, layout::{LayoutEngine, GlyphMetrics, WrapPlan, PositionedLine}, state::{TextLayerState, TextLayerConfig, TextItem}, writing::WritingMode}` | `lib.rs` が全モジュール `pub mod`。`ResolvedBalloonText::resolve(&BalloonModel, (u32,u32))`・`region`／`font`／`choice_style` は `pub`。`TextSlotBinding::new(Entity, Entity, f32, (u32,u32), (u32,u32))`・`scale`／`image_size` は `pub`。`DWriteMetrics::new(&IDWriteFactory2, &ResolvedFont, WritingMode, &TextLayerConfig) -> Result<_, TextLayerError>`（`draw.rs` が再輸出）。`LayoutEngine::layout(&[TextItem], usize, &TextRegion, WritingMode, f32, &dyn GlyphMetrics, WrapPlan)`・`visible_window`。`GlyphMetrics` に `advance`／`line_pitch`／`line_box_height`。`DEFAULT_FONT_NAME = "ＭＳ ゴシック"`・`TextLayerConfig::line_pitch = font_height + 2` | 一致 |
| 檻 H の `highlight_band_extent(12, 12, 14)`・`cursor.style,square` → `SquareFill` | `choice.rs` `pub fn highlight_band_extent(f32, f32, f32) -> f32`・`ResolvedChoiceStyle::SquareFill`（square 明示は SquareFill）。ただし設計の Allowed Dependencies 一覧に `choice` モジュールが**書かれていない**（公開なので到達はする・一覧の書き漏れ） | 一致（軽微） |
| `log_capture_kit::capture`・`wintf::com::dwrite::dwrite_create_factory`・`CoInitializeEx` | `capture<R>(FnOnce() -> R) -> (R, Vec<CapturedEvent>)`・`CapturedEvent` に `level`／`field(name)`。`dwrite_create_factory(DWRITE_FACTORY_TYPE) -> Result<IDWriteFactory2>`。同 crate の既存 5 テストが同じ `CoInitializeEx(None, COINIT_MULTITHREADED)` を使う | 一致 |
| 依存が `crates/areka-emo-text/Cargo.toml` に既在 | `[dependencies]` に `areka-emo-present`・`areka-parsers`・`areka-sakura`・`wintf`・`windows`・`bevy_ecs`、`[dev-dependencies]` に `areka-emo-atlas`・`log-capture-kit` | 一致 |

### (b) ログ捕捉の檻が恒真でないこと

- 捕捉窓は `resolve_balloon_faces`＋`load_scope_balloon_model` に限る（DD3）。`resolve_balloon_faces` は scope≧1 で `ChainTier::Default` へ縮退した面ごとに `tracing::warn!(scope, surface_id, prefix, file, …)` を出す（`balloon.rs` の R6.2 の腕）ので、scope 1 の **`warn!` 2 件ちょうど・欄 `surface_id`∈{2,3}・`prefix`=`balloons`** は正の主張であり、R6.2 の腕を潰せば赤になる。
- scope 0 の `warn!` 0／`error!` 0 は不在主張だが、同じ窓が R6.1 の `info!`（解決完了）を必ず捕らえ、さらに `log-capture-kit` の `run_with_subscriber` が**番兵イベントを捕捉できなければ panic**（`capture.rs`）するので「捕捉 0 件のまま緑」は構造的に起きない。
- 期待値は面の集合・原寸・領域座標といった正の値と組で固定するので、檻 C 単独の恒真化はない。判定: 妥当。

### (c) `spec_checks.rs` 腕 a〜f と台帳変更

- 腕 a: `count = 28` ＝ `[[spec]]` 28 行——設計どおり。腕 b: `.kiro/specs/areka-P0-default-balloon-bundle/` は直下に実在。腕 c: `owner_count = 1` は台帳 4 本で本 spec を宛先に持つ項目 1 件と一致。腕 d: `linkage.md` に `[bundle."絵の重ね方"]` が実在（`linkage.md:2414`）。腕 e: `[[owner_completed]]` は触らない。腕 f: 宛先が `[[spec]]` に載る。`SpecRow` の欄 `name`／`stage`／`bundle`／`owner_count`／`wave` は `documents/parse.rs` が読む欄と一致。**腕 a〜f は緑になる。**
- しかし `cargo test -p ukadoc-survey` は腕 a〜f だけではない——下の Critical Issue 1。

## Critical Issues

🔴 **Critical Issue 1**: `briefing.md` の `[[barrier]]` 行が赤になるのに変更ファイル一覧に無い
**Concern**: 台帳の `ukadoc:descript_balloon:use_self_alpha_2c_5024:1` を `absent`→`degraded` へ変えると、`doc/ukadoc-coverage/briefing.md` §4-2 の `[[barrier]] page = "descript_balloon"`（`degraded = 6`・`absent = 123`）が台帳の数え直しと 1 ずつ食い違う。この行は `crates/ukadoc-survey/tests/consistency/briefing_arms.rs` の `distribution_findings`（腕 f・「ページの全項目を数える」）が状態別に機械照合しており、`the_spec_table_agrees…` と同じ `cargo test -p ukadoc-survey` で赤になる。設計の C5「台帳 3 文書」・File Structure Plan・要件 8.5 の「触れてよいファイル」のいずれにも `briefing.md` が無く、要件 6.5（番人が緑）と 8.5（列挙外に触れない）が両立しない。記憶にも「台帳を触ると briefing/roadmap-draft の手書きの数が赤」（balloon-font-descript-keys）として前例がある。
**Impact**: 実装で番人が赤のまま報告を作るか、8.5 違反として `briefing.md` を無断で触るかの二択になる。
**Suggestion**: 設計ディスカッションで「`briefing.md` の `[[barrier]] descript_balloon` の `degraded` を 6→7・`absent` を 123→122 に直す 2 数値だけを本 spec の変更に加える（8.5 の列挙に `briefing.md` を足す）」と決め、C5 の順序を「台帳 → `roadmap-draft.md` → `briefing.md` の 2 数値 → 報告 → 検査」に改める。同節の散文（§4-2「`descript_balloon` … は `degraded` が 0 でない」・§4-3「0 の欄が 15」）は 6→7・123→122 で真のまま変わらない（設計時に確認済み）。あわせて `roadmap-draft.md:336` の束「絵の重ね方」の「依存する既存 spec」欄（現在 `areka-P0-shell-parse`（完了・1 件）のみ）へ本 spec（A0・1 件）を書き足す——機械照合はされないが「表示するだけの数は必ず古びる」の再発を避ける。
**Traceability**: 要件 6.2・6.5・8.5
**Evidence**: design.md「C5 台帳 3 文書」・「File Structure Plan / Modified Files」・「Requirements Traceability 6.5」

（到達不能と判断して落とした候補: `[stage.X]`・`[[after]]`・`[priority_blank]`・`[tally]` の各数は優先度か対象 4 状態の和で数えるので `absent`→`degraded` で動かない。`briefing.md` §7 の表「縮退 2」は `[[template]]` の語彙との積で、`use_self_alpha` はテンプレートの語彙に無いので動かない。）

## Design Strengths

1. **経路の同一性が構造で担保されている**: 檻 C／D は起動側 `emo2_boot/assets.rs` と同じ `resolve_balloon_faces` → `build_balloon_target_from_faces`（`UseSelfAlpha::On` 固定）を踏み、檻 E は本番 `present_frame` と同じ `ResolvedBalloonText::resolve` を踏む。起動時と別の近道を作っていないので、緑が実機の表示保証に直結する。
2. **期待値の出所が全部実測か導出で `research.md` §8.2 に紐付き、較正の扱い（檻 F・G の導出値は実測へ合わせ理由を記録・それ以外は緩めない）が Error Strategy に明文化されている**。DD3 の「件数固定＝保管フォルダの変化の検出器」という意図も要件 2.2 と整合する。

## Final Assessment

**Decision: GO（条件付き）**

**Rationale**: 設計の骨格（資産＋外付けの檻・本番 0 行・公開 API のみ）は実測で全部裏が取れ、実装経路は明確である。唯一の到達可能な赤は台帳側の `briefing.md` 2 数値で、設計ディスカッションで文書の変更範囲を 1 本足す裁定を入れれば解消し、アーキテクチャの変更を伴わない。

**Next Steps**:
1. `/kiro-design-discussion` で Critical Issue 1 を裁定（`briefing.md` の 2 数値を変更範囲に加える・8.5 の列挙へ追記・C5 の順序を改める・`roadmap-draft.md:336` の欄を足す）。
2. 軽微: Allowed Dependencies に `areka_emo_text::choice::{highlight_band_extent, ResolvedChoiceStyle}` を書き足す。
3. 裁定後 `/kiro-spec-tasks areka-P0-default-balloon-bundle`。
