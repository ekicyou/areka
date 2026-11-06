# ギャップ分析: placement-crate-split

- 測った日: 2026-11-06。測った木: ワークツリーの枝 `claude/kiro-start-placement-crate-split-9da94b`（`42a6df3e`＝main `73c5b25d` に spec の初期化を 1 つ足しただけ・配置のコードは main と同じ）。
- 測り方: Grep・Glob・Read と、使い捨ての python の数え上げ（コメントを除いて `placement::…` の道を use の木ごと開き、`pub(crate)` の宣言と突き合わせた）。数は「コンパイラに数えさせる前の見積り」で、設計の段の確定のしかたは 6 章に書いた。
- 要件（requirements.md）と spec.json は確定済みとして扱い、書き換えていない。ここに書くのは事実と案で、決めは要件の討議と設計の段に回す。

---

## 1. 要約

- **移す形そのものは素直**: 配置は 160 ファイル。本番から本体のほかの部分への `crate::` は 0、外の crate の使い方も要件 1.7 の一覧どおり（areka-actor・dola ほかは使っていない）。新しい crate の骨組み・`git mv`・中の `crate::placement::` の書き換え（81 ファイル・243 か所）・本体の書き換え（169 ファイル）は機械の作業で済む。
- **一番大きい食い違い＝テストの支えの開け方**: 要件 3.1 は `capture_logs`・`LogEvent`・`ExpectField`・`expect_one`・`emo2_root`・`balloon_root` を `test-support` 機能で開くと書くが、これらは `log-capture-kit`・`sample-ghost-kit`・`temp-path-kit` を使う。Cargo の機能が足せるのは本番の依存（optional）だけで、開発用の依存は機能から引けない。ところが既存の番人 `with_default_guard_test.rs` の `the_shared_crate_never_appears_in_a_production_dependency_table` と `sample_path_guard_test.rs` の `the_sample_gateway_never_appears_in_a_production_dependency_table` が、その 2 つの crate を本番の依存表に書くと赤にする。要件のままでは組めない。
- **境をまたぐテストは 3 本ではない**: コードで本体を使うテストは要件 3.5 の 3 本のほかに `measure_bake_count_tests.rs`・`measure_bake_log_tests.rs`（どちらも `crate::emo2_boot::assets` と measure の私有の関数 `measure_with_reader`・`measure_baked` を使う）の 2 本がある。本体のファイルを文字で読むテストも 4 本ある（`measure_template_tests`・`spawn_zorder_pair_deferred_tests`・`zorder_group_branch_coverage_tests`・`zorder_inversion_branch_coverage_tests`）。
- **公開面の数は brief の見積りより多い**: 外から使われる `pub(crate)` のモジュールは 11 でなく 13（`balloon_syncscale`・`face_windowposition` が抜けていた）、本番の項目は約 28 でなく約 43、`follow` の再公開は 4 でなく 19。外から使われない `pub(crate)` は約 31 でなく約 42。
- **性能の判定の道具は出所の名前に頼っていない**（要件 6.7 の答え）: `judge-perf.py` は起動時の DPI の行を `"primary_dpi=" in line`、表示の行を文言で拾う。`judge-followup.py` も文言で拾う。直すのは説明の文だけ。ただし `tools/perf/fixtures-loop/` の `run.log` には作り手（生成器）が無い（`generate.py` は `fixtures/` しか作らない）ので、要件 5.6 の「`generate.py` で作り直す」は `fixtures-loop/` には当てはまらない。

---

## 2. 今の姿

### 2.1 配置のファイルと行き先

| まとまり | ファイル数 | 行き先 |
|---|---|---|
| 配置の本番・テスト・テストの支え（下の残る分を除く全部） | 約 141 | `crates/areka-placement/src/` |
| `#[cfg(test)]` の判定器の一族（`transition_judge*.rs` 13・`transition_signoff_tests.rs`・`transition_signoff_procedure_tests.rs`） | 15 | 本体のテスト専用のモジュール（要件 3.6） |
| 本体を使うテスト（要件 3.5 の 2 本＝`reset_balloon_pos_tests.rs`・`spawn_assembly_tests.rs`。3 本目の `transition_judge_tests.rs` は上の一族に入る） | 2 | 本体に残す |
| 本体を使うテスト（要件に無い分＝`measure_bake_count_tests.rs`・`measure_bake_log_tests.rs`） | 2 | **未定**（2.4 節） |

- 合わせて 160 ファイル（直下 150・`follow/` 10）。
- `transition_judge.rs` の中の `#[path]` は兄弟のファイルを指すので、一族は同じフォルダへまとめて移せば相対の道は変わらない。`transition_judge.rs` は 999 行で、`use super::diag` を `use areka_placement::diag` に書き換えると rustfmt の折り返しで 1,000 行を超えるおそれがある（要件 1.9 の例外表）。
- 配置の根 `mod.rs` は 971 行。規約の文（要件 7）を書き足すと 1,000 行に近づく。`PlacementError` を別のファイルへ移す案（2.5 節 ④）を採れば約 60 行減る。

### 2.2 外から使われる公開面（測り直し）

数え方: 本体の `src`（配置を除く）・`examples`・`tests` と、本体に残す 17 ファイルの中の道を開き、配置の `pub(crate)` の宣言と突き合わせた。`how`・`reason`・`as_str` のような短い名前は目で確かめた。

| 種類 | brief の見積り | 測り直し | 内訳 |
|---|---|---|---|
| `pub(crate) mod` → `pub mod` | 11 | **13** | brief の 11＋`balloon_syncscale`（`ghost_session.rs` が `register` を呼ぶ）・`face_windowposition`（`emo2_boot/frame/balloon_face.rs`） |
| 本番の項目（関数・型・定数・メソッド） | 約 28＋メソッド 1 | **約 43** | `balloon_limit` 2・`balloon_lock` 6（`BalloonLockLedger::reason` を含む）・`balloon_syncscale` 1・`dpi_sync` 8・`face_windowposition` 1・`follow` の中の定義 15・根と `windowposition` 3（`scale_signed`・`load_scope_windowpositions`・`primary_monitor`）・`reseed` 3・`source` 1（`read_kv_lenient`）・要件 2.5 で連れて開く 3（`DpiSyncOutcome`＝`evaluate` の戻り値、`ChainPlacement`＝`place_chain_in_work_area` の戻り値、`ChainClampOutcome`＝`ChainPlacement` の公開のフィールドの型） |
| フィールド | ― | **7** | `DpiSyncHold.since_frame`（外で構造体を組む）・`DpiSyncOutcome` の 4 つ・`DefaultPlacementBasis.{source, balloon}`（外で構造体を組む） |
| 再公開（`pub(crate) use` → `pub use`） | 4 | **20** | `follow.rs` の 19 名（`follow_sticky_move`・`log_sticky_ignored`・`OffsetRescale`・`rescale_follow_offset` は定義が既に `pub` で、再公開の行だけ変わる）・根の `scale_signed` 1 |
| テストの支え（`#[cfg(test)]` の中身） | ― | **約 18** | 2.3 節 |

- `follow` の再公開は、定義の側も `pub` でないと `pub use` にできない（E0364）。そのため再公開 19 名のうち 15 名は、`follow/*.rs` の定義の `pub(crate)` も一緒に `pub` にする（上の「本番の項目」の 15 に数えた）。
- 外から一度も使われない `pub(crate)` は **約 42**（brief は約 31）。`balloon_limit` 9・`balloon_lock` 3・`balloon_syncscale` 1・`dpi_sync` 4・`follow` の中 19・根 2・`reseed` 3・`zorder_group_ledger` 1。私有のモジュール `windowposition` の 4 つは数え方しだいで入る・入らないが変わる（外からはもともと見えない）。要件 2.2 のとおり残す。
- ほかの公開の道（`placement::follow::OffsetBase` を 23 ファイルが書くなど）は既に `pub` で、書き換えは道の先頭だけ。

### 2.3 テストの支えと番人（要件 3.1〜3.4 の食い違い）

外のテストが使う配置のテストの支えと、それぞれが頼る crate:

| 支え | 外の使い手 | 頼る crate | `test-support` 機能で開けるか |
|---|---|---|---|
| `test_support`（`capture_logs` 38 ファイル・`LogEvent` 25・`ExpectField` 8・`expect_one` 4） | 本体のテスト約 40 ファイル＋本体に残す 2 本 | `log-capture-kit`（中身は `log_capture_kit::capture` の薄い包み） | **開けない**（`log-capture-kit` を本番の依存表に書くと `with_default_guard_test` が赤） |
| `shared_test_support` の `emo2_root`・`balloon_root` | 本体のテスト 12・50 ファイル | `sample-ghost-kit` | **開けない**（`sample_path_guard_test` が赤） |
| `shared_test_support` の `TempDir`・`synth_declared_dpi_ghost`（`pub(super)`） | 本体に残す `reset_balloon_pos_tests.rs` | `temp-path-kit` | 番人は無いが、本体の `Cargo.toml` の説明は「[dependencies] へは決して置かない」と書く |
| `shared_test_support` の `WA`・`with_com_initialized`・`emily_shaped_scopes` | 本体のテスト数本 | 配置の型・`windows` だけ | 開ける |
| `reseed::tests` の `booted_world`・`new_shell`・`char_positions` | `emo2_boot/frame/switch_finish_tests.rs` ほか 2 | 上の `shared_test_support`・`test_support` を引く | 上の 2 行に引きずられる |
| `spawn::test_support`（私有の `mod`・`pub(super)` の 3 関数） | 本体に残す `spawn_assembly_tests.rs` | 配置の型・wintf だけ | 開ける（`pub(super)` を開く手間が要る） |
| `GhostTitles::from_scope_titles`（`#[cfg(test)]` の `impl`） | 本体のテスト 22 ファイル | 無し | 開ける |
| `persist::project_restore`（`#[cfg(test)] pub fn`） | 本体のテスト 1 | 無し | 開ける |

- Cargo の機能は本番の依存（optional）しか有効にできず、開発用の依存（dev-dependency）を機能から引く手は無い。配置の crate の `#[cfg(test)]` は、本体のテストを組むときには立たない（立つのは配置の crate 自身のテストだけ）。
- 結論: 要件 3.1 の一覧のうち上の 2 行は、番人の決まり（`log-capture-kit` と `sample-ghost-kit` は本番の依存表に出さない）を変えない限り `test-support` 機能では開けない。要件 3.4（`capture_logs` の汎用の部分を `log-capture-kit` へ下ろすか）の答えが、ここで実質的に決まる（6 章の議題 1）。
- 番人の表の書き換え（要件 5.1）: `file_length_guard_test.rs` 3 件（`follow/window_move.rs` は配置の crate へ、`transition_judge_tests.rs`・`transition_judge_verdict_tests.rs` は**本体の**テスト専用のモジュールへ）・`temp_path_guard_test.rs` 2 件（`measure_tests.rs`・`placement_monitor_tests.rs`。brief は 1 件と書いていた）・`with_default_guard_test.rs` 3 件。`with_default_guard_test.rs` の理由欄は「areka は当該フィーチャを有効にしない」と書いており、主語が `areka-placement` に変わる。

### 2.4 境をまたぐテスト

**コードで本体を使うもの（配置の crate に置けない）**

| テスト | 本体のどこを使うか | 配置のどこを使うか |
|---|---|---|
| `reset_balloon_pos_tests.rs`（要件 3.5） | `emo2_boot::switch_assets` | 公開の項目＋`shared_test_support` の `TempDir` ほか |
| `spawn_assembly_tests.rs`（要件 3.5） | `input_events::attach_char_pointer_handlers` | `spawn::test_support`（私有）・`follow::on_balloon_drag_end` ほか |
| `transition_judge*` の一族（要件 3.5・3.6） | `emo2_boot::target_map` | `diag`・`transition_diag`・`dpi_sync::DPI_SYNC_HOLD_MAX_FRAMES`（公開にする分で足りる） |
| **`measure_bake_count_tests.rs`（要件に無い）** | `emo2_boot::assets::{build_balloon_assets, build_shell_assets}` | **私有の** `measure_with_reader`・`measure_baked`・`measure_scope_sizes` |
| **`measure_bake_log_tests.rs`（要件に無い）** | 同上 | **私有の** `measure_with_reader` |

- 下の 2 本を本体に残すと、measure の私有の関数 2 つを外から呼べるように開くことになる（要件 2.1「外から使われている項目だけ」の外）。配置の crate に置くと、本体の `emo2_boot::assets` を呼べない。どちらかを選ぶ必要がある（6 章の議題 2）。

**本体のファイルを文字で読むもの（`include_str!`・`CARGO_MANIFEST_DIR`）**

| テスト | 読む先 | 移した後 |
|---|---|---|
| `measure_template_tests.rs` | `../emo2_boot/{frame_test_support,assets_tests,assets}.rs`・`../../examples/{emo-present/setup,collision-probe/setup,window-placement}.rs`・私有の `build_shell_assets`・`compose_size` を使う | 配置の crate に置くなら `../../areka/src/emo2_boot/…`・`../../areka/examples/…`・`../examples/window-placement.rs` に書き換え |
| `spawn_zorder_pair_deferred_tests.rs` | `CARGO_MANIFEST_DIR` からの `src/placement/*`・`src/emo2_boot/*` の名簿と、`src` の木の走査 | 配置の crate に置くと走査が配置の `src` しか見なくなり、本体の `zorder_` の本番ファイルが増えても気づかない（静かに狭まる）。名簿の各項目の実在の検査は赤になるので、黙っては通らない |
| `zorder_group_branch_coverage_tests.rs` | `../emo2_boot/zorder_cue*.rs`・`../emo2_boot/frame/zorder_*.rs` | どちらに置くかで道が変わる |
| `zorder_inversion_branch_coverage_tests.rs` | 上に加えて `../../../wintf/src/ecs/window/*`（深さが 1 つ変わる） | 同上 |
| `transition_signoff_procedure_tests.rs`（本体に残る） | `src/placement/transition_judge_offset_signoff_tests.rs` ほか | 本体のテスト専用のモジュールの道へ |

- `include_str!` の先が無いとコンパイルが止まり、`read_to_string` の失敗は `panic!` になる＝要件 5.3 の「黙って通らない」は既に満たしている。危ないのは木の走査（`spawn_zorder_pair_deferred_tests` の `all_source_files`）だけ。
- 配置の中で兄弟のファイルを読むもの（`follow_transition_diag_tests` ほか）と、隣の crate を読むもの（`measure_bake_count_tests` の `../areka-emo-compose/…`・`zorder_property_deferral_tests` の `../areka-sylphya/src`）は、`crates/` の深さが同じなので移しても道は変わらない。
- 下流: `runtime-module-untangle` が `emo2_boot` を改名すると、配置の crate に置いた文字読みのテストの道が切れる（コンパイルで赤になる＝気づける）。

### 2.5 依存の向きの規約と実際

規約の文（`mod.rs` 8〜14 行）は `resolver` ← `config` ← `measure` ← `spawn`／`follow` ← main.rs シーム、`diag` は最下流、と 6 つのモジュールだけを名指す。配置のモジュールは 27 ある（本番の `.rs` と `follow`）。実際の参照（本番のファイルの `use super::…`・`super::super::…`・`crate::placement::…`）:

| 要件 7.2 の件 | 実際 | 移すだけで直るか |
|---|---|---|
| ① `resolver` が `tracing`・`wintf::ecs::DPI`・`config`・`follow::OffsetBase` を引く | 確かめた（19〜23 行）。`config` は `resolver` を引かない＝実際の向きは `resolver` → `config`。`config.rs` は `areka_parsers` を引くが（3 か所）、`resolver` は引かない | **一部だけ**。`OffsetBase` は `PointPx` と `Option<DPI>` だけの値の型（`follow/offset_space.rs` 158 行）で、`resolver.rs` へ移して `follow` から `pub use` で出し直せば `resolver` → `follow` は消える（外の 23 ファイルの道は変わらない）。`tracing`・`DPI`・`config` は移しても消えない＝規約の文を書き直す |
| ② `diag` が `bevy_ecs::Entity`・`wintf::ecs::ZOrderPairStrategy` を引く | 確かめた（60〜62 行）。配置のモジュールは 1 つも引かない＝「最下流」は正しい | **直らない**（他の crate の型）。規約の文を書き直す |
| ③ `config` が `tracing` を引く | 確かめた（11 行） | **直らない**。規約の文を書き直す |
| ④ `measure` が根の `PlacementError` を引く | 確かめた（50 行）。**`source.rs`（22 行）も同じく引く**（要件の一覧に無い） | **直る**。`PlacementError`（`MountError`・`PathBuf`・`io::Error`・`thiserror` だけを使う）を独立のファイルへ移し、根から `pub use` で出し直す。`source` も同時に直る |
| ⑤ `spawn` と `follow` が互いに引く | 確かめた | 同じ段なので規約の文に書き足す |

- **要件の 5 件の外にある「根を引く子」**: `reset_pos.rs` が `super::primary_monitor`（根の関数）、`follow/offset_space.rs` が `crate::placement::scale_signed`（根が `windowposition` から出し直したもの）を引く。どちらも ④ と同じ形（根 ↔ 子の参照し合い）。`scale_signed` は `crate::windowposition::scale_signed` を直に指せば移すだけで消える。`primary_monitor` は置き場を移せば消える。
- **規約が名指さない 21 モジュールの参照**は密で、輪が多い（`follow` ↔ `balloon_limit`・`dpi_sync`・`persist`・`spawn`・`sticky_group_ledger`・`transition_diag`、`chain_finalize` ↔ `chain_realign` ほか）。要件 7.1「文が実際の参照と一致する」を、名指す 6 つの間だけで読むか、27 全部で読むかで仕事の量が大きく変わる（6 章の議題 4）。
- 規約の文の「`config`（areka-parsers のみ）」の部分も、`config` が `tracing` を引くので正確でない（③ と同じ）。

### 2.6 ログの出所

- 本番のログは既定の出所（モジュールのパス）を使うので、移すと `areka::placement::…` が `areka_placement::…` に変わる。直書きの `DIAG_TARGET = "areka::placement::diag"`（`diag.rs` 69 行）を合わせる（要件 6.2）。
- 名前で読む所（要件 6.5 の「ほか」の中身・測り直し）:
  - `diag_tests.rs`（32・509・519 行）・`follow_window_move_diag_tests.rs`（547 行）
  - **`measure_bake_count_tests.rs` 359 行の `MEASURE_TARGET = "areka::placement::measure"`・`measure_bake_log_tests.rs` 294 行**（brief の一覧に無い。捕まえたログを出所の名前で選ぶテスト）
  - `crates/areka/tests/window_link_real_run.rs` 49 行・wintf の `ecs/window_proc/window_pos_tests.rs` 14 行（`RUST_LOG`）
  - 説明の文: `diag.rs` 15・67・136 行、wintf の `ecs/window_proc/lifecycle.rs` 34 行、`tools/perf/judge-perf.py` 695 行、`examples/window-placement.rs` 103 行
- 変えなくてよいと確かめた所:
  - `log_history.rs` の `RULES` は `areka::placement` を名指さない（`areka` は完全一致だけ）。配置の info は今も残らず、警告以上は出所に関わらず error の種別＝移しても同じ。
  - `doc/ssp-mcp/log-convention.md`・`real-run-kit` の `launch.rs`・`doc/` には `areka::placement` が無い。
  - `EnvFilter` の名指しは素の前方一致なので、`areka_placement=debug` は `areka_placement::diag` も開く（要件 6.3）。
  - 配置の中の `"areka::persist::*"` などの語彙の出所は変えない（要件 6.4）。
- `tools/perf/fixtures/*/run.log`（20 ファイル）と `fixtures-loop/*/run.log`（21 ファイル）は `INFO areka::placement: 起動時 DPI 構成 …` の行を持つ。

### 2.7 性能の判定の道具（要件 6.7 の答え）

- `judge-perf.py`: 起動時の DPI の行は `if not scan.boot_dpi and "primary_dpi=" in line`（1052 行）、表示の行は `J_SHOW_LINE_MESSAGE in line`（1000 行）で拾う。**出所の名前に頼っていない**。名前が出るのは 695 行の説明だけ。
- `judge-followup.py`: 文言（`MSG_TOGGLE`・`MSG_SHOW`・`TRANSITION_TAG` ほか）で拾う。**頼っていない**。112 行の説明が `crates/areka/src/placement/diag.rs:338-343` を名指す（行番号つき）。
- `perf-rank.py`・`perf-compare.py`・`judge_perf_catchup.py`・`perf_rank_dump.py` と `.ps1` の `RUST_LOG` には `areka::placement` が無い。
- よって要件 6.8（直して SelfTest を回す）は発動しない見込み。ただし見本の作り直しで run.log が変わるので、SelfTest を回して緑を示すのは要件 5.6 の確かめとして要る。
- **`fixtures-loop/` には作り手が無い**: `generate.py` は `fixtures/` だけを作る（`boot_line` は `primary_dpi=96` 固定）。`fixtures-loop/rank/full_ok/run.log` は `primary_dpi=192` の行を持ち、`generate.py` からは作れない。git の履歴でも手で置いた見本（`327e7fd3`）。要件 5.6 の「手で書き換えず作り直す」は `fixtures-loop/` には当てはまらない（6 章の議題 5）。

### 2.8 パスの書き換え先（要件 5）

| 場所 | 件数（`src/placement` の出現） | 備考 |
|---|---|---|
| `doc/COMPAT_ARCHITECTURE.md` | 37（25 行） | brief の「25 か所」は行の数 |
| `doc/ukadoc-coverage/ledger/assets.toml` | 19（6 行） | 散文。証拠の道は検査がソースから毎回集めるので、台帳の作り直しは要らない（`ukadoc-survey` は `crates/**/*.rs` を全部歩く＝新しい crate も自動で入る） |
| `doc/ukadoc-coverage/ledger/sakura-script.toml` | 8 | 同上 |
| `doc/ukadoc-coverage/briefing-assets.md` | 13 | |
| `.kiro/steering/structure.md` | 271 行の `placement/` の説明ほか | crate の一覧に `areka-placement` の節が要る。415 行の「`log-capture-kit` を使う 18 crate」の数も動く見込み |
| 説明の文（wintf 4・`pilot` 1・`ukadoc-survey` 1・本体の `emo2_boot` 2・`tools/perf/judge-followup.py` 1） | 各 1〜2 | wintf の `runtime/tick_bridge.rs` が名指す `transition_signoff_tests.rs` は**本体の**テスト専用のモジュールへ行く。`frame_transition_atomicity_tests.rs` の `window_move.rs:632` と `judge-followup.py` の `diag.rs:338-343` は行番号つき |
| 配置の中で自分の古い道を書くもの | 3 ファイル | `spawn_zorder_pair_deferred_tests`・`transition_judge_offset_signoff_tests`・`transition_signoff_procedure_tests` |
| `audit-placement-core`・`audit-placement-follow` の brief | 各 1 行 | `audit-placement-follow` の持ち物 `transition_*` は、`transition_diag*` が配置の crate、`transition_judge*`・`transition_signoff*` が本体に分かれる |

### 2.9 そのほかの気づき

- **`windows` の機能（要件 1.7）**: ワークスペースの `[workspace.dependencies.windows]` は `Win32_System_Com`・`Win32_System_LibraryLoader` を含む 27 の機能を最初から持つ。慣習の `windows = { workspace = true }` では機能を足せても減らせない。要件 1.7 の「テストだけで使う機能は組むときにだけ付ける」は、`workspace = true` のままでは字面どおりにできない（6 章の議題 6）。ほかの crate（`areka-emo-atlas`・`real-run-kit` ほか）も `workspace = true` で全部の機能を受けている。
- **開発用の依存の機能は試験のビルドで本番の側にも混ざる**: `resolver = "2"` では、`cargo test`・`cargo check --all-targets` のように開発用の依存を組むときに限り、本体の dev-dependency が付けた `areka-placement/test-support` が同じビルドの本体の bin にも効く。`crates/areka/tests/*.rs` の実機テストが起こす本体の exe には、テストの支えのコードが入った配置の crate が入る（振る舞いは変わらない・リリースのビルドには入らない）。
- **lib にすると死んだコードの判定が変わる**: lib の `pub` 項目は死んだコードの警告から外れ、`pub(crate)` で crate の中から使われないものは警告になる。今の bin で警告 0 なので、外から使われるものを `pub` にすれば新しい警告は出ない見込み（コンパイルで確かめる）。逆に、配置の本番に 51 個ある `#[allow(dead_code)]`・`#[allow(unused_imports)]` の多くは「`#[path]` で取り込む examples では使い手がいない」「areka は lib を持たない bin」を理由にしており、移した後は理由の文が古くなる（`transition_diag.rs` 16・`follow.rs` 11・`follow/work_area_clamp.rs` 10 ほか）。属性を外すと隠れていた死んだコードが出るおそれがある。
- **`private_interfaces` の警告**: `pub` にした関数が `pub(crate)` の型を返すと警告になる（Rust 2024 の既定）。2.2 節の「連れて開く 3」はその見込みで、型の別名 `ScopeWindowPositions`（`BalloonPlacementInputs` の公開のフィールドの型）が引っかかるかはコンパイルで確かめる。
- **文書のテスト**: lib になると doc コメントの中のコードが文書のテストとして回る。配置の本番の doc にある囲みは `transition_diag.rs` の ```` ```text ```` 1 つだけで、字下げのコードも無い＝新しい文書のテストは 0 の見込み。テストの名前の一覧（要件 8.2）にも増えない。
- **`THIRD-PARTY-NOTICES.md`**: ワークスペースの crate も並ぶ（`areka-emo-atlas 0.1.0` など）ので、作り直すと `areka-placement 0.1.0` の行が増える。`license.workspace = true` が要る。
- **落ちの報告と MTA の初期化子（要件 3.8・3.9）**: 本体の `emo2_boot/crash_report_test_support.rs`（588 行・許可の持ち手の表つき）と wintf の `test_crash_report.rs`（494 行・表なし）。配置の crate は GPU の許可の持ち手を持たないので、写しの元は wintf の形が近い。`.CRT$XCU` の初期化子は配置の crate 自身のテストの実行ファイルにだけ入る（`#[cfg(test)]`）。
- **`tools/load-flake.ps1`**: `ValidateSet('areka-bin', 'wintf-lib')` と 209 行の `if` の 2 か所を直せば `-Binary` に足せる。
- **examples**: `window-placement.rs` は `sample_ghost_kit`・`tracing_subscriber`・`wintf`・`areka_emo_*` を使う＝配置の crate の開発用の依存に `sample-ghost-kit`・`tracing-subscriber` が要る（どちらもワークスペースの中か `Cargo.lock` に既にある）。`collision-probe.rs` は `#[path]` で `emo2_boot/target_map.rs`・`hit_region.rs` も取り込み、その 2 ファイルは配置を説明の文でしか名指さない＝配置を crate の依存に替えるだけで済む。
- **ほかのウェーブの兄弟との接触**: 2.3 節の議題 1 で `log-capture-kit` の `src`（ないし `sample-ghost-kit`）に手を入れる案を採ると、roadmap の C18 の「② が触る本体の外のファイル」の一覧の外に出る。C18 の兄弟（③〜⑧）の受け持ちには `log-capture-kit/src`・`sample-ghost-kit` は入っていない（`audit-sylphya-actor` が触らないと約束したのは `temp_path_guard_test.rs` が追う一時の入口だけ）。

---

## 3. 要件と資産の対応表

タグ: **Missing**＝今は無く作る／**Unknown**＝設計で調べる／**Constraint**＝既存の決まりが縛る／**OK**＝今の資産で足りる。

| 要件 | 今の資産 | タグ | メモ |
|---|---|---|---|
| 1.1〜1.6 crate の新設・`git mv`・根を `lib.rs`・path 依存・根の `Cargo.toml` 不変 | `members = ["crates/*"]`・ほかの crate の `Cargo.toml` の型 | OK | 慣習の型は `areka-emo-atlas` などに揃える |
| 1.7 本番の依存と `windows` の機能 | 外の crate の使い方は一覧と一致 | **Constraint** | `workspace = true` では機能を減らせない（2.9） |
| 1.8 本体に依存しない | 本番の `crate::` 0 | OK | テストの 5 本＋一族が本体に残れば満たす |
| 1.9 1,000 行 | 番人は `crates/` を全部歩く | **Constraint** | `transition_judge.rs` 999・根 971 の余裕が小さい |
| 2.1・2.2 外から使われる分だけ `pub` | 2.2 節の数 | **Unknown** | 確定はコンパイルに数えさせる（6 章） |
| 2.3・2.4 自分を指す道・本体の道の書き換え | 81 ファイル 243 か所／169 ファイル | OK | 機械の置換 |
| 2.5 連れて開く型の記録 | ― | **Missing** | 見込み 3 つ |
| 3.1〜3.3 `test-support` 機能・写しを作らない | `test_support`・`shared_test_support` | **Constraint** | 番人 2 つと衝突（2.3） |
| 3.4 `log-capture-kit` へ下ろすか | `log_capture_kit::capture`・`CapturedEvent` | **Unknown** | 議題 1 の答えしだい |
| 3.5 境をまたぐテストを本体に | 3 本は確認 | **Missing** | 他に 2 本（`measure_bake_*`）が私有の関数と本体の両方を使う |
| 3.6・3.7 判定器を本体のテスト専用のモジュールへ | `#[cfg(test)]`・兄弟の `#[path]` | OK | 置き場の名前は未定 |
| 3.8・3.9 MTA と落ちの報告の 3 つ目の写し | 本体・wintf の 2 つ | **Missing** | wintf の形が近い |
| 3.10 `load-flake.ps1` | `ValidateSet` と `if` | **Missing** | 2 か所 |
| 4.1〜4.3 examples | `#[path]` の 2 本 | **Missing** | 配置の crate の開発用の依存に `sample-ghost-kit`・`tracing-subscriber` |
| 5.1 番人の表 | 3＋2＋3 件 | OK | 判定器の 2 件は本体の道へ |
| 5.2・5.3 文字で読むテスト | 2.4 節 | **Unknown** | 置き場の決め（議題 3）と、木の走査の扱い |
| 5.4・5.5・5.7 文書・説明・brief | 2.8 節 | OK | 行番号つきの説明が 2 つ |
| 5.6 見本の作り直し | `generate.py` は `fixtures/` だけ | **Constraint** | `fixtures-loop/` に作り手が無い（議題 5） |
| 5.8・6.6 grep 0 件 | ― | OK | 5.6 の扱いしだいで `tools/` が 0 にならない |
| 6.1〜6.5 出所の名前 | 2.6 節 | OK | `measure_bake_*` の 2 か所を足す |
| 6.7・6.8 性能の道具 | 2.7 節 | OK | 頼っていない＝6.8 は発動しない見込み |
| 7.1〜7.5 規約 | 2.5 節 | **Unknown** | 名指さない 21 モジュールの扱い（議題 4）・根を引く子が 5 件の外に 3 つ |
| 8.1〜8.8 検証 | 道具は揃っている | OK | 多重集合の比べは crate が変わっても末尾の名前で比べるので安定 |

---

## 4. 実装の案

### 案 A: 別名で道を保ったまま移す（今の部品を延ばす）

- 新しい crate へ移し、本体の `main.rs` に `use areka_placement as placement;` を 1 行置く。本体の 169 ファイルの `crate::placement::…` は書き換えずに通る。`collision-probe.rs` も同じ 1 行で済む。
- ✅ 差分が一番小さく、移動の差分と書き換えの差分が混ざらない。
- ❌ 要件 2.4（`areka_placement::…` へ書き換える）を満たさない。古い道 `placement::` が本体に残り、`runtime-module-untangle` 以降の分け方で同じ別名が増えていく。
- 使い道: 最終形にはできないが、**途中の 1 コミット**（移した直後にビルドを緑にする）としては使える。

### 案 B: 新しい crate へ一度に移して全部書き換える

- 骨組み → `git mv` → 中の道 → 公開面 → 本体の道 → テストの置き場 → examples → 表・文書・出所、を 1 本の流れで行う。
- ✅ 要件どおりの最終形。中間の形が残らない。
- ❌ 途中でビルドが長く赤のまま。公開面の数え違いをコンパイルが一度にまとめて出す（数百件のエラー）ので、レビューの単位が大きい。

### 案 C: 段に分けて移す（組み合わせ）

1. 骨組みと `git mv` と中の道の書き換え（`crate::placement::` → `crate::`）＋本体に案 A の別名を一時に置き、`pub(crate)` → `pub` をコンパイルのエラー（E0603 私有の項目・E0616 私有のフィールド・E0624 私有のメソッド・E0364 再公開）に数えさせて開く。ここで公開面の数が確定する（要件 2.1・2.5 の記録を作る）。
2. 本体の道を `areka_placement::…` へ書き換えて別名を外す（要件 2.4）。
3. テストの置き場（判定器の一族・境をまたぐテスト）と `test-support` 機能・テストの支え。
4. examples・番人の表・文字で読むテスト・出所の名前・文書・見本・`load-flake.ps1`・MTA の写し。
5. 検証（多重集合・警告・全体テスト・実機 1 本・`cargo about generate`）。

- ✅ 各段で `cargo check --workspace --all-targets` を緑にでき、レビューも段ごとにできる。公開面の数をコンパイラが確定するので、手の数え違い（brief の 11／28／4 のような）が起きない。
- ❌ 段の間に一時の別名が 1 コミットだけ存在する（最終形には残らない）。

---

## 5. 規模と危うさ

- **規模: L（1〜2 週）**。ファイルの数は多いが大半は機械の置換。手のかかるのは、テストの支えの開け方（番人との衝突）・境をまたぐテストの置き場・規約の文・見本の作り直し・実機 1 本。brief の 13〜16 タスクの見積りは、議題 1〜4 の答えによって 1〜3 タスク増える。
- **危うさ: 中**。技術は既知（Cargo の機能・crate の分け方）で、振る舞いを変えない。ただし、⑴ 番人 2 つの決まりと要件 3.1 が衝突しており、答えによって `log-capture-kit`（ないし番人の表）に手が及ぶ、⑵ 境をまたぐテストが要件の数え（3 本）より多い、⑶ 1,000 行の余裕が 1 行と 29 行のファイルがある、⑷ テストが新しい実行ファイルに分かれて落ちる組み合わせが変わる（要件 3.8・3.9 の写しで手当てする）。

---

## 6. 設計の段への申し送り

### 決めること（要件の討議の議題の候補）

1. **テストの支えの開け方**（要件 3.1〜3.4・最重要）。`log-capture-kit`・`sample-ghost-kit` を本番の依存表に書けない番人がある。案:
   - (a) `capture_logs`・`LogEvent`・`ExpectField`・`expect_one` を `log-capture-kit` へ下ろし（要件 3.4 を「下ろす」へ）、`emo2_root`・`balloon_root` は使う側が `sample_ghost_kit::SampleRoot` を直に呼ぶ（中身は 1 行の包み）か `sample-ghost-kit` へ下ろす。配置の型と `windows` だけで書ける支え（`WA`・`with_com_initialized`・`emily_shaped_scopes`・`spawn::test_support`・`from_scope_titles`・`project_restore`）だけを `test-support` 機能で開く。写しは 0。`log-capture-kit` の `src` に触る（C18 の触るファイルの一覧の外）。
   - (b) 番人の決まりを変えて、`areka-placement` だけが `test-support` 機能の下で optional の依存として持つことを許す（`with_default_guard_test`・`sample_path_guard_test` の表と、本体の `Cargo.toml` の「決して置かない」の決まりの文を書き換える）。コードの差分は小さいが、決まりそのものを変える。
   - (c) 本体に同じ薄い包みをもう 1 つ置く。要件 3.3（写しを作らない）に反する。
2. **`measure_bake_count_tests.rs`・`measure_bake_log_tests.rs` の置き場**。本体の `emo2_boot::assets` と measure の私有の関数の両方を使う。案: (a) 本体に残し、私有の 2 関数を `test-support` 機能の下でだけ開く（要件 3.5 に 2 本を足し、要件 2.1 の例外として記録する）、(b) テストを 2 つに割り、本体を使う部分だけ本体へ、(c) 配置に残して `emo2_boot::assets` の呼び出しを公開の口で置き換える（テストの中身が変わるので「移すだけ」を超える）。
3. **本体を文字で読むテスト 4 本の置き場**（要件 5.2）。案: (a) 配置の crate に残し `../../areka/…` を読ませる（私有の関数を開かずに済む・下の crate が上の crate の文字を読む形になる・`runtime-module-untangle` で道が切れたらコンパイルで赤）、(b) 本体へ移す（「触る範囲を含む一番下の crate」の原則に合う・`measure_template_tests` と `spawn_zorder_pair_deferred_tests` は私有の項目や `spawn::test_support` を開く必要が出る）。どちらでも `spawn_zorder_pair_deferred_tests` の木の走査は両方の crate の `src` を歩くように直す（静かに狭まらないように）。
4. **規約の文の範囲**（要件 7.1）。名指す 6 モジュールの間だけで一致させるか、27 全部の段を書き下ろすか。後者は輪が多く、段の表にすると「同じ段どうしは引き合ってよい」の段が大きくなる。あわせて、要件 7.2 の 5 件の外にある根を引く子 3 つ（`source` → `PlacementError`・`reset_pos` → `primary_monitor`・`follow/offset_space` → `scale_signed`）を 7.3 の「移すだけで直す」に入れるか。① の `OffsetBase` を `resolver` へ移すか（移すだけで済むことは確かめた）。
5. **`tools/perf/fixtures-loop/` の見本**（要件 5.6・5.8・6.6）。作り手が無い。案: (a) 出所の名前の 1 語だけを機械で置換する（`areka::placement:` → `areka_placement:`・行の他は変えない）ことを「手で書き換えない」の例外として認める、(b) `fixtures-loop/` 用の作り手を書く（規模が大きく、移すだけを超える）、(c) 古い記録の形の見本として残し、grep の除外に `tools/perf/fixtures-loop/` を足す（道具は出所の名前を読まないので判定は変わらない）。
6. **`windows` の機能の書き方**（要件 1.7）。`workspace = true` では減らせない。案: (a) 慣習どおり `workspace = true` にして、要件 1.7 の機能の部分を「本番では一覧の機能しか使わない（ソースの上で）」と読み替える、(b) 配置の crate だけ版と機能を直に書く（慣習から外れ、版の上げ下げを 2 か所で揃える手間が増える）。
7. **判定器の一族の置き場の名前**（要件 3.6）。本体の中の新しいテスト専用のモジュール（例: `crates/areka/src/transition_judge/`）を `main.rs` で `#[cfg(test)] mod` として宣言する形が素直。名前は `runtime-crate-split` で `areka-runtime` へ移すときにもそのまま使えるものがよい。
8. **古くなる `#[allow(...)]` と理由の文**（51 個）。移すだけの spec では残し、理由の文の「bin」「examples の `#[path]`」の書き直しも含めて `audit-placement-core`・`audit-placement-follow` へ申し送るか、この spec で理由の文だけ直すか。

### 調べること（設計の段）

- 公開面の確定は、案 C の段 1 でコンパイラに数えさせる（E0603・E0616・E0624・E0364・`private_interfaces`）。この表の数（13・約 43・7・20・約 18）は見積り。
- 型の別名 `ScopeWindowPositions` が `pub` のフィールドの型に使われるときに `private_interfaces` の警告が出るか。
- `transition_judge.rs`（999 行）と根（971 行）が書き換えと rustfmt の後に 1,000 行を超えないか。超えるなら例外表に足すか、`PlacementError` を移す案で根の行を減らすか。
- 本体の dev-dependency の `test-support` が試験のビルドで本体の bin にも効く（2.9）ことが、実機テスト・`areka-test-binary-stability` の測りの前提に響かないか。
- テストが新しい実行ファイルに分かれた後、`cargo test --workspace -j 4` の同時に回る実行ファイルの数が増えることが、揺れるテストの出方に響かないか（要件 3.8・3.9 の写しのほかに要るものが無いか）。

---

（次の手: 要件の討議で 6 章の議題 1〜8 の答えを決め、要件 3.1〜3.5・5.6・7.1〜7.3・1.7 を必要なら書き換えてから `/kiro-design placement-crate-split` へ。）
