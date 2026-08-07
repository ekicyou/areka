# Gap Analysis: areka-P0-file-slimming

> 実施 2026-08-07 / 対象ブランチ `claude/areka-p0-file-slimming-64d065`（`9afcabc`・作業ツリー clean）
> 入力: `requirements.md`（確定済・本書は改変しない）／`brief.md`／`.kiro/steering/{product,tech,structure,workflow,logging,focus}.md`
> 計測は本ブランチの `crates/*/src/**/*.rs` 全数走査（`#[cfg(test)]` ブロックの波括弧を文字列・raw 文字列・行/ブロックコメントを除外して追跡する自前スキャナ）で再取得した。

---

## 1. Analysis Summary

- **檻はすべて「ファイル末尾」に在り、interleaved 檻は事実上存在しない。** 檻を持つ 200 ファイル中、檻の後ろに本番コードが残るのは `crates/wintf/src/ecs/world/mod.rs` の 5 行（`impl Debug for EcsWorld`）**1 件のみ**。48 本の必須対象では **0 件**。したがって「檻を本体の途中へ挿入すると後続行の全アンカーがずれる」という要件冒頭の主因記述は、**現在のレイアウトでは成立していない**（後述 §3.1・要件討議で再定義が要る）。移設は「末尾切り出し」に還元でき、実装リスクは想定より低い。
- **移設方式は 3 案あり、既存前例と「本番ファイルのパスを変えるか」で明確に分かれる。** 素の `#[cfg(test)] mod tests;`＋`foo/tests.rs`（**本番ファイル移動 0 本**）／`#[cfg(test)] #[path="foo_tests.rs"] mod tests;`（**同 0 本**・`src/` に前例 0）／ディレクトリモジュール化 `foo/mod.rs`＋`foo/tests.rs`（**同 43 本**）。3 形式とも実機 rustc 1.97.1 で動作確認済（§4）。テスト名（`placement::follow::tests::*`）はいずれの案でも**変わらない**。
- **要件 1.3（複数檻を「同一のテストファイル」へ集約）と要件 2.4（テスト名不変）は 7 ファイルで衝突する。** 複数檻ファイルは 7 本（最大 `crates/areka/src/main.rs` の 7 檻）。1 ファイルへ入れ子集約するとモジュールパスが `tests::tests::*` へ変わりテスト名が変化する。名前を保つには「檻モジュール 1 つ＝ファイル 1 つ」（＝同一**ディレクトリ**への集約）にせざるを得ない。裁定が要る。
- **`#[cfg(test)]` の非 `mod` 項目 44 件は移設できない性質のものが大半。** 内訳は `use` 9／`fn` 10＋`pub*` 系 19（テスト専用アクセサ・`impl` 内メソッド）／`struct` 1／`impl` 2／構造体フィールドと分岐 3。`impl` ブロック内の inherent メソッド（`frame.rs:300-345` の 5 件等）は本体側に残すしかない。要件 1.6 の裁定は「原則残置」に落ちる見込み。
- **本体分割（follow.rs／frame.rs）だけは機械作業でない実質リスクを持つ。** ① `follow.rs` 本番本体は `crate::` パスを**1 件も使っていない**（`examples/window-placement.rs:107`・`collision-probe.rs:231` が `#[path="../src/placement/mod.rs"]` で include するため意図的）——分割後のサブモジュールも同じ規律を守る必要がある。② サブモジュール化は `tracing` の既定 target（＝モジュールパス）を `areka::placement::follow::<sub>` へ変える。`RUST_LOG` の前置一致では吸収されるが、target 完全一致で判定する檻があれば壊れる（follow/frame の檻には無いことを確認済・他 spec の手順書は要確認）。

---

## 2. Current State Investigation（全数実測 2026-08-07）

### 2.1 規模（`crates/*/src/**/*.rs`）

| 指標 | 値 | 要件記載値 | 差 |
|---|---:|---:|---|
| 対象ファイル数 | 387 | 387 | 一致 |
| 総行数 | 189,190 | 189,190 | 一致 |
| `#[cfg(test)] mod {...}` 行数合計 | **92,868**（49.1%） | 92,591（48.9%） | **+277** |
| 檻 > 500 行のファイル | **48** | 48 | 一致 |
| 上記 48 本の檻合計 | **66,830** | 66,830 | 一致 |
| 檻 > 1,000 行のファイル | 26 | 26 | 一致 |

**+277 の出所（要件 1.2 の「乖離の理由」に相当）**: `crates/shiori-host32-host/src/lifecycle.rs:272` の `#[cfg(test)] pub(crate) mod tests { ... }`（277 行）。前回計測スクリプトの正規表現が `pub\s+mod` しか見ておらず `pub(crate) mod` を取りこぼしていた。当該檻は 277 行＝500 行閾値未満のため **48 本の必須対象一覧は不変**。同型の可視性付き宣言は他に 3 件（すべて宣言のみ・§2.3）。

檻の形式内訳（全 387 ファイル）:

| 形式 | 件数 |
|---|---:|
| in-file 檻 `#[cfg(test)] mod X { ... }` | 228 |
| in-file 檻 `#[cfg(test)] pub(crate) mod X { ... }` | 1（lifecycle.rs） |
| 外部ファイル宣言 `#[cfg(test)] mod X;` | 52 |
| 外部ファイル宣言 `#[cfg(test)] pub(crate) mod X;` | 3 |
| `#[cfg(test)]` が `mod` 以外に付く項目 | 44 |

### 2.2 必須対象 48 本（クレート別・`総行 / 本番本体 / 檻 / 宣言元の形`）

`PLAIN` = 素の `foo.rs`（ディレクトリモジュール化すればパスが変わる）／`MODRS` = `mod.rs`／`ROOT` = `lib.rs`・`main.rs`。

**crates/areka** — 13 本 / 檻 21,056
| パス（`src/` 以下） | 総行 | 本体 | 檻 | 形 |
|---|---:|---:|---:|---|
| `placement/follow.rs` | 8,472 | 1,996 | 6,476 | PLAIN |
| `emo2_boot/frame.rs` | 4,660 | 1,497 | 3,163 | PLAIN |
| `input_events/balloon.rs` | 2,825 | 829 | 1,996 | PLAIN |
| `placement/mod.rs` | 1,899 | 563 | 1,336 | MODRS |
| `placement/spawn.rs` | 1,582 | 426 | 1,156 | PLAIN |
| `placement/persist.rs` | 1,535 | 465 | 1,070 | PLAIN |
| `placement/resolver.rs` | 1,306 | 295 | 1,011 | PLAIN |
| `emo2_boot/move_cue.rs` | 1,634 | 700 | 934 | PLAIN |
| `placement/measure.rs` | 1,387 | 465 | 922 | PLAIN |
| `main.rs` | 1,842 | 941 | 901 | ROOT |
| `emo2_boot/assets.rs` | 1,225 | 405 | 820 | PLAIN |
| `input_events/mod.rs` | 1,164 | 433 | 731 | MODRS |
| `placement/source.rs` | 819 | 279 | 540 | PLAIN |

**crates/areka-emo-text** — 7 本 / 檻 12,411
| `layout.rs` 3,294/749/2,545 | `viewbox_draw.rs` 3,090/785/2,305 | `actor.rs` 2,967/858/2,109 | `viewbox.rs` 2,498/749/1,749 | `draw.rs` 2,293/963/1,330 | `choice.rs` 1,749/550/1,199 | `state.rs` 1,630/456/1,174 |（すべて PLAIN）

**crates/areka-emo-present** — 4 本 / 檻 7,563: `presenter.rs` 5,417/1,042/4,375・`balloon.rs` 2,264/632/1,632・`cache.rs` 1,100/193/907・`scale.rs` 876/227/649（PLAIN）

**crates/areka-kanade** — 6 本 / 檻 7,150: `schedule/steady.rs` 3,286/903/2,383・`schedule/mod.rs` 2,176/681/1,495（MODRS）・`schedule/boot.rs` 1,406/288/1,118・`actor.rs` 1,318/370/948・`shiori/real.rs` 903/280/623・`schedule/events.rs` 993/410/583

**crates/areka-sakura** — 2 本 / 檻 3,824: `drive.rs` 2,808/530/2,278・`compile.rs` 1,867/321/1,546

**crates/areka-emo-compose** — 3 本 / 檻 3,715: `plan.rs` 2,203/667/1,536・`scale.rs` 1,778/467/1,311・`fold.rs` 1,132/264/868

**crates/areka-seriko** — 3 本 / 檻 3,469: `actor.rs` 2,331/484/1,847・`state.rs` 1,576/518/1,058・`looper.rs` 939/375/564

**crates/areka-ghost** — 3 本 / 檻 2,903: `dispatcher.rs` 1,856/420/1,436・`runtime.rs` 1,613/650/963・`ticker.rs` 823/319/504

**crates/wintf** — 4 本 / 檻 2,520: `ecs/window_proc/window_pos.rs` 1,160/444/716・`ecs/clickthrough/controller.rs` 1,092/455/637・`ecs/window_proc/dpi_helpers.rs` 746/148/598・`ecs/layout/systems/monitor_systems.rs` 1,050/481/569

**crates/areka-sylphya** — 1 本 / 檻 866: `actor.rs` 1,587/721/866
**crates/dola** — 1 本 / 檻 758: `cue/command.rs` 1,089/331/758
**crates/shiori-host32-helper** — 1 本 / 檻 595: `main.rs` 1,114/519/595（ROOT）

**形の内訳（要件 3.5 の「パスが変わるファイル」の母数）**: PLAIN **43** / MODRS **3** / ROOT **2**。
**クレート数 12**（＝要件 7.1 のコミット粒度は最大 12 論理コミット。`areka-parsers`・`areka-actor`・`areka-talk`・`shiori-abi`・`shiori-host32-{ipc,host,testdll}`・`shiori4-testdll`・`pilot` は対象 0 本）。

### 2.3 既存前例（要件 3.4 の対比材料）

`crates/*/src/` に `#[path = ...]` は **1 件も無い**（`#[path]` の 24 件はすべて `crates/wintf/tests/*.rs` の統合テスト入口と `crates/*/examples/`・`crates/pilot/examples/`）。
一方、素の `#[cfg(test)] mod <name>;`（宣言のみ・実体は別ファイル）は **55 箇所 / 26 宣言ファイル**（要件記載の 52 箇所は `pub(crate) mod` 3 件を除いた数）。既存の実体ファイル配置は 2 系統ある:

| 系統 | 実例 | 宣言元 | 檻ファイル |
|---|---|---|---|
| **(a) 同一ディレクトリ・フラット** | `areka-parsers/src/sakura/{model,lexer,decode,parse,validation}_tests.rs`（5×5 モジュール）・`areka-emo-compose/src/{composer,golden,log_firing}_tests.rs`・`areka/src/shiori_{,lifecycle_,reference_}e2e_tests.rs`・`areka-sylphya/src/ledger_key_determinism_tests.rs` | `mod.rs` / `lib.rs` / `main.rs` | 宣言元と同じディレクトリ |
| **(b) ディレクトリモジュール `{module}/tests.rs`** | `dola/src/runtime/{instance_manager,interpolator,loop_controller,subscription_manager,timeline_manager}/tests.rs`・`wintf/src/ecs/{drag/state,graphics,layout/hit_region,layout/hit_test,pointer/dispatch,pointer/types,widget/bitmap_source,widget/text/typewriter,window/window_pos}/tests.rs`・`areka/src/emo2_boot/spine.rs` | `{module}/mod.rs` | `{module}/tests.rs` |

**(a) は宣言元が `mod.rs`/`lib.rs`/`main.rs`（＝ディレクトリの module root）だから `#[path]` 無しで成立している。** 素の `foo.rs` からフラットな `foo_tests.rs` を引くには `#[path]` が要る——これが 43 本の PLAIN に効く分岐点。

**steering の既存記述**（`structure.md` §Test Naming Conventions / L142-145）:
> #### Unit Tests (in-source `#[cfg(test)]`)
> - **Inline**: 小規模テストはソースファイル内に `mod tests { ... }` として記述
> - **Separated**: `{module}/tests.rs` — ディレクトリモジュール化パターン（`bitmap_source/` を参照）

さらに `structure.md` L204:
> **モジュール分割パターン**: 600行リファクタ（`oversized-file-refactor`）以降、肥大化したファイルは `{module}/mod.rs` + サブモジュールのディレクトリ形式へ分割する方針。dola `runtime/` がその代表例。

→ 要件 6.1/6.2 の追記先はこの 2 箇所が正本。**現行 steering は (b) を「分離時の型」として既に名指ししている**ため、(b) を採らない裁定をするなら steering 側の書き換えが必要（単なる 1 行追記では済まない）。

**`foo.rs` と `foo/` の共存前例も既にある**: `crates/areka-emo-atlas/src/decode.rs`（`pub mod wic_arm;`）＋ `crates/areka-emo-atlas/src/decode/wic_arm.rs`。Rust 2018+ で合法であることが本リポジトリ内で実証済み。

### 2.4 檻の位置構造（本 spec の実装難度を決める最重要実測）

檻を持つ **200 ファイル**について「最初の檻が始まった行より後に、檻に属さない非空・非コメント行が残るか」を全数走査した結果:

- **該当 1 ファイルのみ**: `crates/wintf/src/ecs/world/mod.rs`（檻の後ろに `impl std::fmt::Debug for EcsWorld` 5 行）。当ファイルは檻 500 行以下＝必須対象外。
- **必須対象 48 本では 0 件**。すべての檻が**ファイル末尾に連続して並んでいる**。
- 檻が複数あるファイルは 48 本中 **7 本**（すべて末尾に連続配置）:

| ファイル | 檻数 | 檻モジュール名（行範囲・行数） |
|---|---:|---|
| `areka/src/main.rs` | 7 | `startup_window_tests`(899-1068,170) `seam_tests`(1077-1292,216) `config_input_tests`(1294-1360,67) `ghost_wiring_tests`(1368-1443,76) `restore_seam_tests`(1453-1578,126) `persist_wiring_seam_tests`(1589-1677,89) `monitor_snapshot_seam_tests`(1686-1842,157) |
| `areka/src/emo2_boot/move_cue.rs` | 4 | `tests`(671-948,278) `move_sink_tests`(954-1069,116) `apply_move_tests`(1081-1429,349) `move_severity_log_tests`(1444-1634,191) |
| `shiori-host32-helper/src/main.rs` | 4 | `resolve_param_tests`(517-568,52) `classify_tests`(570-660,91) `load_ack_tests`(662-689,28) `loopback_tests`(691-1114,424) |
| `areka-emo-text/src/choice.rs` | 3 | `tests`(537-1121,585) `style_resolve_tests`(1129-1339,211) `decorate_tests`(1347-1749,403) |
| `areka-sylphya/src/actor.rs` | 3 | `tests`(675-1028,354) `actor_integration_tests`(1036-1379,344) `actor_criteria_cage`(1420-1587,168) |
| `areka-emo-text/src/actor.rs` | 2 | `tests`(858-939,82) `runtime_tests`(941-2967,2027) |
| `areka-kanade/src/schedule/mod.rs` | 2 | `tests`(670-1554,885) `log_firing_tests`(1567-2176,610) |

**含意（要件 1.3 との衝突）**: これら 7 本を「同一のテストファイル」へ入れ子集約すると、テスト名が `move_cue::tests::x` → `move_cue::tests::tests::x` のように変わる。要件 2.4（テスト名を変更しない）と正面衝突する。名前を保つなら「檻モジュール 1 つ＝檻ファイル 1 つ」（同一**ディレクトリ**への集約）が唯一解。

### 2.5 `#[cfg(test)]` 非 `mod` 項目 44 件（要件 1.6 の対象・全数）

| 分類 | 件数 | 実例（file:line） | 移設可否の見立て |
|---|---:|---|---|
| `impl` ブロック内のテスト専用 inherent メソッド | 19 | `emo2_boot/frame.rs:300,315,327,336,345`（`drain_move_directives`/`read_back_target`/`drain_received`/`apply_present`/`balloon_model_scopes`）・`shiori_host.rs:308`・`areka-emo-text/src/{segment.rs:76,surface.rs:380,draw.rs:429,893}`・`wintf/src/ecs/widget/bitmap_source/{systems.rs:48,task_pool.rs:90,96}`・`wintf/src/runtime/window_registry.rs:98`・`dola/src/runtime/subscription_manager/mod.rs:103`・`shiori-host32-{helper/src/main.rs:417,423, host/src/parent_window.rs:344,350}` | **移設不可**（inherent impl は本体側にしか置けない。`impl` を檻ファイルへ切り出せば「本体の私有フィールドへ触るメソッドを別ファイルで定義」となり成立はするが、可視性の緩和を招く） |
| 自由関数（テスト専用ヘルパ） | 10 | `input_events/mod.rs:84`（`with_clock`）・`areka-emo-text/src/draw.rs:929,942` 他 | 移設可能だが本体側の型に密着 |
| `use` 宣言（テスト時のみ必要な import） | 9 | `areka-emo-text/src/draw.rs:74,76,105,107,109,111,113,115,117`（全 9 件が同一ファイルに集中） | **本体側残置が自然**（`DrawExecutor` 等のテスト専用型が本体に在るため） |
| `struct` / `impl` | 3 | `shiori_host.rs:291`（`TestSylphyaSink`）・`areka-emo-text/src/draw.rs:539,691,706`（`FormatKey`/`DrawExecutor`/`impl`）・`placement/source.rs:76`（`impl GhostTitles`） | 移設可能（型自体を檻ファイルへ移せる）だが本体からの参照が生じると壊れる |
| 構造体フィールド＋分岐 | 3 | `areka-emo-text/src/viewbox_draw.rs:116,146,153,484`（`fail_next_render` フィールド／初期化／注入 fn／`if self.fail_next_render`） | **移設不可**（フィールドと分岐は本体の内部状態） |

**「毒化」との関係**: `viewbox_draw.rs` の `fail_next_render` は本番構造体に埋め込まれた注入シームであり、[[obsolete-vs-broken-test-policy]]／要件 5.1（時刻注入シームの変更禁止）に照らして**本 spec では触れない**のが正しい。`test-cage-determinism`（W6.9）へ所見として送る候補。

### 2.6 follow.rs 本番本体 1,996 行の責務シーム（要件 4.1）

| 行範囲 | 責務 | 主要項目 | 概算 |
|---|---|---|---:|
| 65-230 | **アンカー射影ポリシー** | `pub trait DragPositionPolicy` / `pub struct BottomSnapPolicy` / `impl` / `pub fn project_anchor` / `pub struct Anchored` | ~166 |
| 232-900 | **ドラッグ＋バルーン追従** | `pub struct BalloonFollow` / `on_char_drag` / `on_char_drag_end` / `policy_mapped_position` / `BalloonFollowTrigger` / `follow_balloon` / `guard_balloon_position` / `on_balloon_drag` / `on_balloon_drag_end` | ~670 |
| 904-1475 | **窓移動・リサイズ API と反映** | `pub fn move_window_to` / `pub fn resize_window_to` / `pub fn anchor_changed_system` / `enqueue_window_set_pos` / `log_window_move` | ~572 |
| 1476-1588 | **モニタ work area 解決** | `pub struct MonitorSnapshot` / `pub fn work_area_for_window` / `pub enum WorkAreaResolution` / `pub fn work_area_for_window_with_origin` | ~113 |
| 1589-1926 | **可視性ガード** | `pub enum VisibilityVerdict` / `pub fn guard_visibility` / `rect_at` / `rects_intersect` / `intersects_any_work_area` / `clamp_x_into` / 3 定数タグ / `route_applies_visibility_guard` / `apply_visibility_guard` / `evaluate_visibility_guard` | ~338 |
| 1927-1996 | 補助 API | `pub fn resize_window_keep_position` | ~70 |

→ 5 シーム。最大は「ドラッグ＋バルーン追従」670 行で、要件 4.2 の目安 1,000 行を全シームが満たす。

**制約（重要）**: `follow.rs` の**本番本体は `crate::` パスを 1 件も使っていない**（`awk` 全数確認・0 件）。これは `crates/areka/examples/window-placement.rs:107` と `collision-probe.rs:231` が `#[path = "../src/placement/mod.rs"] mod placement;` で placement 木ごと include するためで（[[areka-examples-path-include-no-crate-paths]]）、`follow.rs:1927` に「examples が `#[path]` include するため、本体未使用ビルドでも必要」という `#[allow(dead_code)]` 注記も残る。**分割後のサブモジュールも `crate::` 不使用を守らねば example のビルドが壊れる。**

`follow::` の外部参照は 26 箇所（`main.rs` 6・`placement/mod.rs` 4・`placement/spawn.rs` 3・`placement/persist.rs` 3・`emo2_boot/move_cue.rs` 3・`emo2_boot/frame.rs` 3・`examples/collision-probe.rs` 3・`examples/window-placement.rs` 1）。`follow` を facade に `pub use` 再輸出すれば呼び出し側は **0 箇所変更**で済む（要件 4.3 を満たす最短経路）。

### 2.7 frame.rs 本番本体 1,497 行の責務シーム（要件 4.1）

| 行範囲 | 責務 | 主要項目 | 概算 |
|---|---|---|---:|
| 64-180 | **attach 計画** | `pub struct AttachPlan` / `pub struct PlannedAttach` / `pub fn plan_attachments` | ~117 |
| 181-368 | **配線コンテナ** | `pub struct Emo2Wiring` ＋ `impl`（**300-345 に `#[cfg(test)]` メソッド 5 件が内在**） | ~188 |
| 369-621 | **attach 相** | `pub fn run_attach_phase` / `connect_balloon_text` | ~253 |
| 622-1044 | **DPI 相** | `AuthorDpis` / `GhostWindowKind` / `GhostWindowClass` / `classify_ghost_window` / `reconcile_window_size` / `trait ScaleReportSource` / `DpiChangedQuery` / `dpi_phase_with` / `reproject_char_window_at_current_size` / `pub fn run_dpi_phase` | ~423 |
| 1045-1191 | **テキスト scale 相** | `pub fn run_text_scale_phase` / `reconcile_reported_sizes` | ~147 |
| 1192-1258 | **drain 相** | `pub fn run_drain_phase` / `pub fn run_move_drain_phase` | ~67 |
| 1259-1407 | **resnap** | `resnap_from_sizes` / `resnap_shell_targets` / `trait PhysicalSizeSource` / `resnap_with` | ~149 |
| 1408-1465 | **テキスト相** | `resolve_talk_time` / `pub fn run_text_phase` | ~58 |
| 1466-1497 | **フレーム統合** | `pub fn emo2_frame_system` | ~32 |

→ 9 シーム（brief の「7 フェーズ」より細かい）。`frame::` の外部参照は 10 箇所（`input_events/balloon.rs`・`input_events/mod.rs`・`tests/emo2_real_run.rs` 等）。

**制約**: `Emo2Wiring` の `impl` に `#[cfg(test)]` メソッドが 5 件混ざっており、`Emo2Wiring` を別サブモジュールへ移すと、それらのメソッドが触る私有フィールドの可視性を `pub(super)` 等へ緩めるか、`impl` を同じサブモジュールへ同伴させる必要がある（要件 2.5「公開 API 不変」はクレート外観測が基準なので `pub(crate)`/`pub(super)` の内部調整は許容範囲だが、design で明示すべき）。

### 2.8 ログ target とモジュールパスの結合（要件 2.7「挙動変更ゼロ」への含意）

`tracing` の既定 target はモジュールパスであり、本リポジトリは `RUST_LOG` ディレクティブをモジュールパスで書く運用（`structure.md`／`logging.md` L109-118・`RUST_LOG="wintf::ecs::graphics=debug"` 等）。`placement/diag.rs:62` は `pub const DIAG_TARGET: &str = "areka::placement::diag"` を**檻で固定**している（`diag.rs:371` の `assert_eq!`）。

- **檻の移設では target は変わらない**（`follow.rs` の `mod tests` を `follow/tests.rs` へ出しても モジュールパスは `areka::placement::follow::tests` のまま）。→ 安全。
- **本体分割では変わる**（`follow.rs` の項目を `follow/visibility.rs` へ移すと既定 target が `areka::placement::follow::visibility` になる）。`RUST_LOG=...areka::placement::follow=debug` のような**前置一致フィルタは吸収する**が、`target ==` 完全一致で判定する檻があれば壊れる。
  - `follow.rs`／`frame.rs` の檻を全数確認: target 文字列の完全一致判定は **無し**（`frame.rs:1977` の capture layer は `level=` で数えるのみ、`follow.rs:6787` は `EnvFilter` の前置ディレクティブ `areka::placement::diag=debug` を使う）。
  - ただし `wintf/src/ecs/window_proc/window_pos.rs:460` のように複数 target を並べた EnvFilter 文字列が他所にもある。分割対象 2 本の項目が発する target 名を design で列挙し、全 EnvFilter 文字列と突合すること（**Research Needed R-3**）。

### 2.9 移設の「無変更性」を実際に阻む唯一の機械的差分＝インデント

`mod tests { ... }` を別ファイルの module root へ出すと、檻本文は**一律 4 スペースの de-indent** が要る（そうしないと rustfmt 差分と読みにくさが残り、`mod tests {}` を檻ファイル内に再度書けばテスト名が `tests::tests::*` へ変わる）。66,830 行が空白差分として動くため:

- 要件 2.4 の「内容不変」の検証は**空白非依存の比較**（`git diff -w` / 行の `lstrip()` 正規化比較）で行う必要がある。
- 逆に言えば、`lstrip()` 正規化後の完全一致は**極めて強い静的証跡**になる（[[areka-evidence-classes-static-equals-real-machine]] の「静的構造証跡」に相当）。design でこの照合スクリプトを成果物に含めることを推奨。

### 2.10 テスト総数の証跡採取（要件 2.2/2.3）— Windows での実行可能性を実測

本環境 `cargo 1.97.1 / rustc 1.97.1` でスクラッチクレートを作り、`--list` の意味論を実測した:

```
$ cargo test -- --list
     Running unittests src\lib.rs (target\debug\deps\probe-<hash>.exe)
bar::tests::path_form: test
deep::inner::tests::nested_path_form: test
foo::tests::dir_form: test

3 tests, 0 benchmarks
   Doc-tests probe
src\lib.rs - add (line 2): test

1 test, 0 benchmarks
```

確認できたこと:
- `cargo test --workspace -- --list` は**テストを実行せずに全テスト名を列挙**する。`#[ignore]` 付きも列挙される（実行時の `ignored` カウントに依存しない）。
- **doctest も同じ実行で列挙される**（`Doc-tests <crate>` セクション）。
- 出力は `<module path>::<fn>: test` 形式＝**数だけでなく名前集合の一致まで比較できる**。テスト総数一致（要件 2.2）より厳密な証跡になり、要件 2.4 の「テスト名不変」も同時に担保する。
- GPU 実描画・実機依存テストを**走らせない**ため決定論的で、[[areka-defender-rescan-starves-cooperative-test-loops]] のような実行時 flake の影響を受けない。

推奨する証跡採取（PowerShell・移設前／後で同一手順）:

```powershell
cargo test --workspace --no-fail-fast -- --list 2>&1 |
  Select-String -Pattern ': test$|: benchmark$' |
  ForEach-Object { $_.Line } | Sort-Object | Set-Content before.txt
# 移設後に after.txt を同じ手順で採取し
Compare-Object (Get-Content before.txt) (Get-Content after.txt)   # 出力ゼロ＝名前集合完全一致
(Get-Content before.txt).Count; (Get-Content after.txt).Count      # 総数
```

**注意点（design で解消が要る）**:
1. `--list` はビルドを要求する。本ワークツリーには `target/` が無く、初回は**フルコールドビルド**になる（`bevy_ecs` + `windows-rs` 系）。移設前スナップショットの採取タイミングを実装開始前に固定すること。
2. `cargo test --workspace` の全緑判定（要件 2.1）は **i686 の host-32 成果物が先に要る**（[[workspace-test-needs-i686-host32-artifacts]]）。`--list` だけならテスト本体を走らせないので不要だが、要件 2.1 の全緑は別途 i686 ビルド後に取る必要がある。
3. `Doc-tests` 行は `src\lib.rs - add (line 2): test` のように**行番号を含む**。本体分割で doctest の位置が動くと名前が変わる。対象 2 本（`follow.rs`／`frame.rs`）に doctest があるかを design で確認し、あれば名前比較から doctest を分離するか、位置不変を保証すること（**Research Needed R-4**）。
4. `--no-fail-fast` を付けないと、いずれかのターゲットのビルドや列挙が失敗した時点で以降が採取されない。

### 2.11 実装ウェーブの空白（要件 5.4）

`git worktree list` / `git branch` 実測:
- 現行ワークツリー 4 本: `main`(247d48a) / 本ブランチ `claude/areka-p0-file-slimming-64d065` / **`claude/areka-p0-file-slimming-e4f098`（同一 spec の重複ワークツリー・`f657d84`）** / `claude/epic-kepler-bdbee8`(ce7d165＝main の PR#102 相当)。
- **他 spec の実装ブランチは 1 本も走っていない**（W5.95＝実装ウェーブ空白期は成立）。
- ただし同一 spec の重複ワークツリー e4f098 が存在する。着手前にどちらを正とするか確認が要る（**Research Needed R-5**）。
- `.kiro/specs/` 直下の active spec は本 spec を含め 16 本（すべて文書フェーズ）。

---

## 3. Requirements → Asset Map

| 要件 | 対応する既存資産 | ギャップ | タグ |
|---|---|---|---|
| **1.1** 檻 500 行超の全ファイルを外出し | (a)(b) 両系統の前例 55 箇所 / 26 ファイル | 48 本すべてが未適用。`areka`・`areka-emo-*`・`areka-kanade`・`areka-sakura`・`areka-seriko` は前例ゼロ | Missing |
| **1.2** 対象一覧の全数再計測 | 本書 §2.1/§2.2 で完了（48 本・66,830 行・要件値と一致） | 全体合計のみ +277 の乖離あり（原因＝`pub(crate) mod` の取りこぼし・§2.1）。48 本一覧は不変 | ✅ 解消 |
| **1.3** 複数檻の同一テストファイルへの集約 | 7 本が該当（§2.4） | **要件 2.4（テスト名不変）と衝突**。「同一ファイル」を「同一ディレクトリ」に読み替えるか、名前変更を許すかの裁定が要る | **Constraint / 要裁定** |
| **1.4** 既に外出し済のファイルを除外 | `emo2_boot/spine.rs`（2,503 行・親 `mod.rs` から gate）ほか 26 宣言ファイル | 除外一覧そのものは §2.3 で確定済。追加作業なし | ✅ 解消 |
| **1.5** 檻 500 行以下は任意 | 檻を持つ 200 ファイル中 152 本が該当 | 同一ディレクトリ一貫性のための任意移設をどこまで広げるかが未定（例: `placement/` は 7/9 本が対象・残り `config.rs`/`diag.rs`/`windowposition.rs` を揃えるか） | Unknown |
| **1.6** 非 `mod` `#[cfg(test)]` 44 件の裁定 | §2.5 に全数（分類 5 種） | 19 件（inherent メソッド）・3 件（フィールド/分岐）は**構造的に移設不可**。裁定は「原則残置＋例外を列挙」に落ちる見込み | **要裁定** |
| **2.1** `cargo test --workspace` 全緑 | `workflow.md` L35 の Test Gate | i686 host-32 成果物の事前ビルドが前提（既存 DoD）。ワークツリーに `target/` 無し＝コールドビルド | Constraint |
| **2.2/2.3** テスト総数一致の証跡 | 前例なし（過去 spec は `test result: ok` の貼付のみ） | **`-- --list` によるテスト名集合の完全一致比較**を新規に導入するのが最良（§2.10・実測済） | Missing（手段は確立済） |
| **2.4** 檻内容不変 | — | 4 スペース de-indent が 66,830 行で発生。空白非依存比較が必須（§2.9） | Constraint |
| **2.5** 公開 API 不変 | `follow::` 26 参照 / `frame::` 10 参照 | `pub use` 再輸出で呼び出し側 0 変更が可能（§2.6） | Low risk |
| **2.6** `cargo build` 警告非増加 | `follow.rs:149,217,1230,1927` 等の `#[allow(dead_code)]` 群 | 分割で `#[allow]` の適用範囲が変わると新規警告が出うる（example 専用の `resize_window_keep_position` 等） | Constraint |
| **2.7/2.8** 挙動不変・可視性側で解決 | — | 本体分割で `tracing` 既定 target が変化（§2.8）。檻移設のみなら不変 | Constraint |
| **3.1-3.3** 単一方式・規則から一意・in-crate 維持 | (a)(b) 2 系統が併存している現状＝既に「単一方式」ではない | 既存 55 箇所を新方式へ揃え直すか、新規移設分だけ揃えるかが未定 | **要裁定** |
| **3.4** 候補方式の対比記録 | §4 に 3 案を実測付きで整理 | design で採否を記録するのみ | ✅ 材料あり |
| **3.5** パスが変わる本番ファイル全数一覧 | §2.2 の形内訳（PLAIN 43 / MODRS 3 / ROOT 2） | 案 A・案 C では **0 本**、案 B では **43 本** | ✅ 材料あり |
| **4.1-4.6** 本体分割 2 本 | §2.6（follow 5 シーム）・§2.7（frame 9 シーム） | `crate::` 不使用規律（example include）・`Emo2Wiring` 内 `#[cfg(test)]` メソッド・tracing target 変化の 3 制約 | **Constraint（最難所）** |
| **5.1-5.5** 隣接 spec 非侵襲 | 実装ブランチ 0 本（§2.11） | `viewbox_draw.rs` の `fail_next_render` 等、cage へ送る所見の登記先（ファイル or brief）が未定 | Unknown |
| **6.1-6.4** steering 明文化・実測更新 | `structure.md` L142-145（Test Naming Conventions）／L204（モジュール分割パターン） | **現行 steering は `{module}/tests.rs` を分離の型として名指し済**。案 A/C を採るなら追記でなく**書き換え**が要る | Constraint |
| **7.1-7.3** クレート単位コミット | [[areka-commit-as-you-go]] | 対象は 12 クレート＝檻分離 12 コミット＋本体分割 2 コミット（+ steering/brief 1）が自然 | ✅ 材料あり |

### 3.1 要件前提の要検証点（要件討議へ送る）

要件 Project Description は肥大の税を 3 つ挙げているが、実測は 1 つ目を支持しない:

> (1) spec が檻を本体の途中に挿入すると後続行の全アンカーがずれる（実例＝`collision-dpi-hittest` PR#100 のマージコミット `ce86995` が `crates/areka/src/input_events/balloon.rs` を +183/-27 で改変し、`bindoption-exclusivity` brief の監視アンカーが +155 ドリフト）

`git show ce86995 -- crates/areka/src/input_events/balloon.rs` のハンク実測:

| ハンク | 位置 | 増分 | 累積ドリフト |
|---|---|---:|---:|
| `@@ -133,8 +133,24 @@` | 本番本体 | +16 | +16 |
| `@@ -151,7 +167,8 @@` | 本番本体 | +1 | +17 |
| `@@ -276,8 +293,10 @@` | 本番本体 | +2 | +19 |
| `@@ -319,7 +338,8 @@` | 本番本体 | +1 | +20 |
| `@@ -442,7 +462,10 @@` | 本番本体 | +3 | +23 |
| `@@ -478,7 +501,8 @@` | 本番本体 | +1 | +24 |
| `@@ -913,8 +937,9 @@ mod tests` | 檻内 | +1 | +25 |
| `@@ -1026,29 +1051,159 @@ mod tests` | 檻内 | +130 | **+155** |
| `@@ -2303,7 +2458,8 @@ mod tests` | 檻内 | +1 | +156 |

- **本番本体のドリフトは +24 で、檻由来ではない**（本番ロジックの改変そのもの）。
- **+155 は檻の中（旧 1051 行以降）にあるアンカーにのみ効く。** 檻はファイル末尾にあるため、檻の増減が**本番本体のアンカーを動かすことはない**（§2.4 の全数実測と整合）。

→ 本 spec の価値は「本番アンカーのドリフト防止」ではなく、**(a) 檻の中を指すアンカーの安定化、(b) 同一ファイル異ハンク衝突（git merge / 干渉台帳）の削減、(c) 4,000〜8,000 行ファイルの編集・diff・レビュー人間工学**の 3 点に再定義するのが実測に忠実。[[doc-claims-need-file-line-verification]] に従い、要件討議で扱うべき項目（設計判断 #9）。**この再定義は本 spec の実施可否を変えるものではない**——(b)(c) だけでも 66,830 行の分離は正当化される。

---

## 4. Implementation Approach Options（要件 3.4 の対比材料）

3 案とも本環境（rustc 1.97.1）で**実際にコンパイル・列挙して動作確認済**。テスト名はいずれも inline 檻と同一（`<module path>::tests::<fn>`）。

### 案 A: 素の `#[cfg(test)] mod tests;` ＋ `foo/tests.rs`（ディレクトリ子ファイル・**本番ファイル移動なし**）

```rust
// crates/areka/src/placement/follow.rs（パス変更なし・末尾）
#[cfg(test)]
mod tests;
// → crates/areka/src/placement/follow/tests.rs（新規ディレクトリ follow/ に tests.rs のみ）
```

- **前例**: 宣言形式（素の `mod X;`）は 55 箇所で既存。`foo.rs` と `foo/` の共存も `areka-emo-atlas/src/decode.rs` + `decode/wic_arm.rs` で既存。
- **規則の一意性**: 「檻ファイル = 本番ファイルの拡張子を落としたディレクトリ配下の `<檻モジュール名>.rs`」。`mod.rs`/`lib.rs`/`main.rs` は自身がディレクトリ root なので `<同一ディレクトリ>/<檻モジュール名>.rs`。**48 本すべてを 1 つの規則で書ける**。
- **本番ファイルのパス変更**: **0 本**（要件 3.5 の一覧が空＝他 spec のアンカーが一度もずれない）。
- **複数檻**: 檻モジュールごとに 1 ファイル（`move_cue/tests.rs`・`move_cue/move_sink_tests.rs`…）→ テスト名完全保存。要件 1.3 の「同一ファイル」を「同一ディレクトリ」と読む必要あり。
- ✅ 本番ファイル無移動 ／ `#[path]` 不要 ／ steering の既存記述 `{module}/tests.rs` と**表記が一致**（ただし既存は `{module}/mod.rs` 前提）
- ❌ `follow/` のように「tests.rs しか入っていないディレクトリ」が 43 個できる（本体分割する 2 本を除けば 41 個）
- ❌ `foo.rs` + `foo/` の共存はリポジトリ内で 1 例のみ＝レビュアーに馴染みが薄い

### 案 B: ディレクトリモジュール化 `foo/mod.rs` ＋ `foo/tests.rs`

```
crates/areka/src/placement/follow.rs → follow/mod.rs
                                     + follow/tests.rs
```

- **前例**: 最多（dola `runtime/*` 5 本・wintf `ecs/*` 9 本）。**`structure.md` L142-145 が「Separated」の型として名指ししている唯一の形**。L204 の「肥大化したファイルは `{module}/mod.rs` + サブモジュール」方針とも一致。
- **本番ファイルのパス変更**: **43 本**（PLAIN 全数。要件 3.5 の一覧＝この 43 本）。**他 spec の brief アンカーが 43 ファイル分、一度だけ全滅する**（`follow.rs:1234` → `follow/mod.rs:1234`）。
- ✅ リポジトリで最も見慣れた形 ／ steering 無改訂で済む ／ 本体分割（要件 4）と自然に合流（`follow/mod.rs` + `follow/drag.rs` …）
- ❌ 43 本のパス変更＝本 spec が「アンカードリフト税を減らす」ために**一度だけ最大のドリフトを起こす**（自己矛盾に見える。ただし 1 回きり・全数一覧を出せる・W5.95 の空白期に払える）
- ❌ `git` のリネーム検出は効くが、`mod.rs` が 43 個増えると `mod.rs` ばかりのタブ表示になる（`rust-analyzer`/エディタの人間工学が悪化するという Rust 2018 の `mod.rs` 忌避の背景そのもの）

### 案 C: `#[cfg(test)] #[path = "foo_tests.rs"] mod tests;`（フラット兄弟ファイル）

```rust
// crates/areka/src/placement/follow.rs（パス変更なし・末尾）
#[cfg(test)]
#[path = "follow_tests.rs"]
mod tests;
// → crates/areka/src/placement/follow_tests.rs
```

- **`#[path]` の解決規則を実測確認**: インライン `mod` ブロック外の `#[path]` は**宣言元ソースファイルのディレクトリ相対**。`src/deep/inner.rs` の `#[path="inner_tests.rs"]` → `src/deep/inner_tests.rs` で解決することを実行確認済（テスト名 `deep::inner::tests::nested_path_form`）。
- **前例**: 出力ファイル名の型（`*_tests.rs` フラット配置）は `areka-parsers`（20 本）・`areka-emo-compose`（3 本）・`areka/src/shiori_*_e2e_tests.rs`（3 本）で既存。ただしそれらは宣言元が `mod.rs`/`lib.rs` のため `#[path]` を使っていない。**`crates/*/src/` における `#[path]` 属性の前例は 0 件**（要件 3.4 の記載どおり）。
- **本番ファイルのパス変更**: **0 本**。
- ✅ ディレクトリを一切増やさない ／ 檻ファイルが本番ファイルの真横に並ぶ（`follow.rs` / `follow_tests.rs`）＝所在が最も直感的
- ✅ 本体分割との干渉なし（`follow/` を本番サブモジュール専用にできる）
- ❌ `src/` に `#[path]` 前例ゼロ＝新規の規約導入。`#[path]` はモジュール解決の直感を壊すため一般に忌避される
- ❌ `mod.rs`/`lib.rs`/`main.rs`（5 本）では `#[path]` が不要（素の `mod foo_tests;` で足りる）ため、**厳密には「単一方式」にならない**（要件 3.1 に対し「`#[path]` を常に明示的に書く」で形式統一は可能だが冗長）

### 比較表

| 観点 | A: `foo/tests.rs`（素の `mod`） | B: `foo/mod.rs`+`foo/tests.rs` | C: `#[path]` フラット |
|---|---|---|---|
| 本番ファイルのパス変更（要件 3.5） | **0 本** | **43 本** | **0 本** |
| `crates/*/src/` の既存前例 | 宣言形式◎ / `foo.rs`+`foo/` 共存△（1 例） | ◎（14 モジュール） | `#[path]` は **0 件** |
| steering との整合 | 表記一致・前提は要追記 | **完全一致（無改訂可）** | 書き換えが必要 |
| 規則の一意性（要件 3.2/3.1） | ◎（全 48 本 1 規則） | ◎ | △（ROOT/MODRS で `#[path]` が冗長） |
| 複数檻 7 本の扱い | ファイル分割（名前保存） | ファイル分割（名前保存） | ファイル分割（名前保存） |
| 本体分割（要件 4）との合流 | ◎（`follow/` に本体サブと tests が同居） | ◎（最も自然） | ◎（`follow/` を本体専用にできる） |
| 増える空ディレクトリ | 41〜43 個 | 0（本体が入る） | 0 |
| レビュー・可読性 | ○ | ○ | ◎（真横に並ぶ） |
| 移設作業そのもののリスク | 低（末尾切り＋de-indent） | 低＋`git mv` 43 本 | 低 |

**いずれの案でも共通して要る作業**: 檻本文の 4 スペース de-indent（66,830 行）／檻先頭の `use super::*` 等はそのまま有効（438 箇所の `use super::` は移設後も同じモジュール関係を維持）／`use crate::...`（744 箇所）もクレートルート相対なので不変。

### 本体分割（要件 4）の選択肢

| | D1: facade 再輸出型 | D2: 純粋移動型 |
|---|---|---|
| 形 | `follow.rs`（または `follow/mod.rs`）に `pub use drag::*;` 等を置き、外部から見た `placement::follow::X` を維持 | 項目を `follow::drag::X` へ移し、呼び出し側 26 箇所を追随 |
| 要件 4.3 適合 | ◎（可視性同一・呼び出し側 **0 変更**） | ○（「呼び出し側の変更をモジュールパスの追随に限る」を満たす） |
| tracing target | 変化する（実体が移るため）。§2.8 の突合が要る | 同左 |
| `crate::` 不使用規律 | サブモジュールにも波及 | 同左 |
| example `#[path]` include | `placement/mod.rs` 経由で自動追随 | 同左 |

**檻の配置（要件 4.6）**: 分割後 `follow` の檻 6,476 行を ① `follow/tests.rs` 1 本に集約（テスト名 `follow::tests::*` 完全保存・6,476 行の巨大ファイルが残る）か、② サブモジュール単位に分配（`follow/drag/tests.rs` 等・**テスト名が `follow::drag::tests::*` へ変わり要件 2.4 に抵触**）か。**要件 2.4 を厳格に読めば ① 一択**。②を採るなら「テスト名は変わるがテスト総数と本文は不変」という緩和を要件討議で明示的に得る必要がある。

---

## 5. Effort & Risk

| 作業単位 | 規模 | Effort | Risk | 根拠 |
|---|---|---|---|---|
| 檻分離（48 本 / 12 クレート / 66,830 行） | 12 論理コミット | **M（3–7 日）** | **Low** | 全檻が末尾＝末尾切り＋de-indent の機械作業。interleaved 0 件（§2.4）。`--list` 名前集合比較で回帰を即検出できる |
| `follow.rs` 本体分割（1,996 行→5 シーム） | 1 コミット | **S–M** | **Medium** | `crate::` 不使用規律・example `#[path]` include・`#[allow(dead_code)]` 群・tracing target 変化 |
| `frame.rs` 本体分割（1,497 行→9 シーム） | 1 コミット | **S–M** | **Medium** | `Emo2Wiring` impl 内の `#[cfg(test)]` メソッド 5 件と私有フィールド可視性 |
| 証跡採取（前後 `--list` 集合比較 + 全緑） | — | **S** | **Medium** | コールドビルド時間・i686 host-32 成果物の事前ビルド |
| steering 追記 / brief 実測更新 | 1 コミット | **S** | **Low** | `structure.md` L142-145 / L204 の書き換え要否は案の選択次第 |
| **合計** | 15 前後のコミット | **M〜L（5 日〜2 週）** | **Low–Medium** | 案 B を採ると `git mv` 43 本と他 spec アンカー全滅の一度きりコストが乗る |

**最大の非機械的リスク**は本体分割 2 本（要件 4）であり、檻分離（要件 1）ではない。要件 7.3 が両者のコミット分離を求めているのは実測上も正しい。

---

## 6. Research Needed（design フェーズへ持ち越し）

- **R-1**: 案 A/C を採る場合、既存 55 箇所の (a)(b) 2 系統を新方式へ揃え直すか、新規 48 本のみ揃えるか。要件 3.1「全ての移設対象へ同一方式」の「移設対象」の外延（既存分離済ファイルを含むか）。
- **R-2**: 要件 1.5 の任意移設をどこまで広げるか。`placement/`（9 本中 7 本が必須）・`emo2_boot/`（11 本中 3 本が必須）のようにディレクトリ内で混在が残る箇所の一貫性方針。
- **R-3**: 本体分割後の新 target 名（`areka::placement::follow::*` / `areka::emo2_boot::frame::*` の子）を列挙し、リポジトリ全体の `EnvFilter` 文字列・`RUST_LOG` 手順書・実機サインオフ grep 語と突合する（`wintf/src/ecs/window_proc/window_pos.rs:460` 等）。
- **R-4**: `follow.rs` / `frame.rs` および 48 本に doctest が存在するか。存在する場合、`--list` の `src\<file> - <item> (line N): test` は行番号を含むため、名前集合比較の対象から doctest を除外するか、位置不変を担保するかを決める。
- **R-5**: 重複ワークツリー `claude/areka-p0-file-slimming-e4f098`（`f657d84`）の扱い。実装をどちらのブランチで行うか（要件 5.4 の確認手順に含める）。
- **R-6**: `crates/wintf/src/ecs/world/mod.rs` の唯一の「檻の後に本番コード」ケース（`impl Debug` 5 行）を、必須対象外でも先に是正して「全檻末尾」を不変条件として steering に書けるようにするか。
- **R-7**: de-indent 差分の検証スクリプト（移設前ファイルの檻領域と移設後檻ファイルを `lstrip()` 正規化して完全一致を確認）を成果物に含めるか。含めるなら置き場（`scripts/` は現状不在）。
- **R-8**: 要件 2.6（`cargo build` 警告非増加）の基準値をいつ採るか。`#[allow(dead_code)]` の適用範囲が分割で変わる箇所（`follow.rs:149,217,1230,1927`）の扱い。

---

## 7. 設計判断項目（要件ディスカッションへ送る・番号付き）

1. **移設方式の裁定（要件 3.1/3.4/3.5）** — 案 A（`foo/tests.rs`・本番移動 0）／案 B（`foo/mod.rs`・本番移動 43）／案 C（`#[path]` フラット・本番移動 0）。判断軸は「本 spec 自身が一度だけ起こすアンカードリフト 43 本を許容するか」と「steering の既存記述 `{module}/tests.rs` をどこまで正典として尊重するか」。
2. **複数檻 7 本の集約単位（要件 1.3 × 2.4 の衝突）** — 「同一のテストファイル」を字義どおり 1 ファイルにするとテスト名が `tests::tests::*` へ変わる。(i)「同一ディレクトリへの集約」と読み替え檻モジュール 1 つ＝1 ファイルにする、(ii) テスト名変更を許容して 1 ファイルへ入れ子集約する、のいずれか。**(i) 推奨**（名前集合一致という最強の証跡を捨てずに済む）。
3. **非 `mod` `#[cfg(test)]` 44 件の裁定（要件 1.6）** — 19 件（inherent メソッド）と 3 件（フィールド/分岐）は構造的に移設不可。「原則本体残置・移設対象は `mod` 檻のみ」と裁定し、44 件の全数一覧（§2.5）を design に転記するのが最短。`viewbox_draw.rs` の `fail_next_render` は `test-cage-determinism` へ送る所見候補。
4. **本体分割 2 本の檻の配置（要件 4.6）** — ① 分割後も `follow/tests.rs`・`frame/tests.rs` へ 1 本集約（テスト名完全保存・6,476 行の檻ファイルが残る）／② サブモジュール単位へ分配（テスト名が変わる＝要件 2.4 の緩和が要る）。**① 推奨**。
5. **本体分割の形（要件 4.3）** — D1 facade 再輸出（`pub use` で呼び出し側 0 変更）／D2 純粋移動（26+10 箇所を追随）。要件 4.3 は「呼び出し側の変更をモジュールパスの追随に限る」なので両方可。
6. **証跡の強度（要件 2.2/2.3）** — 「テスト総数一致」で足りるとするか、**「テスト名集合の完全一致」（`-- --list` のソート済み差分ゼロ）まで求めるか**。後者は追加コストほぼゼロで、要件 2.4 の一部も同時に担保する。design で採用形式を確定すべき。
7. **檻内容不変の検証方法（要件 2.4）** — 4 スペース de-indent が 66,830 行に必ず入るため、レビューは空白非依存比較（`git diff -w` または `lstrip()` 正規化スクリプト）で行う旨を明記するか。
8. **steering の追記先と書き換え範囲（要件 6.1/6.2）** — `structure.md` L142-145「Test Naming Conventions / Unit Tests」が第一候補。案 A/C を採る場合は既存の「Separated: `{module}/tests.rs` — ディレクトリモジュール化パターン」の記述を**書き換える**必要がある（追記 1 行では矛盾が残る）。L204 の「肥大化ファイルは `{module}/mod.rs` + サブモジュール」も本体分割の形と突合が要る。
9. **要件冒頭の「税(1)」の再定義（§3.1）** — 実測上、檻はすべてファイル末尾にあり、檻の増減が本番本体のアンカーを動かすことはない。PR#100 の +155 ドリフトも本番本体分は +24 のみで、+131 は檻内アンカーに対するもの。本 spec の価値主張を「檻内アンカーの安定化 ＋ 同一ファイル異ハンク衝突の削減 ＋ 巨大ファイルの人間工学」へ書き直すかどうか。**実施可否には影響しない**（(b)(c) だけで 66,830 行の分離は正当化される）が、要件文書の事実性の問題として裁定が要る。
10. **任意移設の範囲（要件 1.5）** — ディレクトリ内で必須対象と非対象が混在する箇所（`placement/` 9 本中 7 本必須・`emo2_boot/` 11 本中 3 本必須・`areka-emo-text/` ほぼ全数必須）で、揃えるか混在を許すか。揃えると対象は 48 本から大きく増える（檻を持つ 200 ファイルが上限）。
11. **重複ワークツリーの整理（要件 5.4・R-5）** — `claude/areka-p0-file-slimming-e4f098` が同一 spec で並存している。実装着手前にどちらを正とするか確定が要る。
