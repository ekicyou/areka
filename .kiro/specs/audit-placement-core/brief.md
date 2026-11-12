# Brief: audit-placement-core

> 2026-10-19 `/kiro-discovery` で起票。開発者「今相談した、ポニーテールによるコードの見直しを、なるべく並行作業が出来るように複数スペックに起票して欲しい。とにかく指摘事項は全部直す方針です。」。同じ日の問いへの答え＝指摘を洗い出すのは各 spec の着手のとき・版の段は「今やる！最優先！」（同じ日のうちに改め＝v0.1.0 の直後の版 v0.1.1 へ移した・開発者「えーっと、ponytailをすぐかけたい！って言ってましたが、0.1.0（ぱすたが動くまで）の直後、0.1.1フェーズに変更してください。」・2026-10-22 さらに改め＝版の段から外し、v0.1.0 の後に隙間で取る優先度が高い列へ移した＝開発者「audit系スペックのロードマップ優先度を、Ver0.1.0が終わった後の、優先度が高いいつでもやれる項目とせよ。基本的に0.1.0が終わった後、隙間で実施したい。」・v0.2.0 の作り直しと重なる分は作り直しの前に済ませる＝roadmap の「版の段」が正本）・範囲は本番のクレート全部と開発の道具のクレートと `tools/` の台本と `pilot`。全 33 本のうちの 1 本で、分け方と共通の約束は roadmap の「テーマ別の決めごと」の「ponytail による見直し」が正本。行数は起票のとき（main `e3c2cbe4`）の実測＝着手のときに測り直す。

## Problem

開発者は、areka のコードに残っているバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードを、一度まとめて直したい。今までは、各 spec が自分の触る所だけを見てきた。全体を同じ物差しで見直したことはない。物差しは ponytail のプラグイン（5.3.2・利用者全体に入れてある）の `/ponytail:ponytail-audit`。この spec は **配置の crate（`areka-placement`）の残り（窓の出し方・設定・保存・重なりの順・DPI ほか）** を受け持つ。

## Current State

- 受け持つファイル: `crates/areka-placement/` のうち `audit-placement-follow` の持ち物でないもの全部（`Cargo.toml`・`examples/` を含む）と、`crates/areka/src/placement_tests/` のうち `transition_*` でないもの（`mod.rs` を含む）。`placement-crate-split`（2026-11-06）が `crates/areka/src/placement/` から移した。
- 大きさ: 114 ファイル・コード 10,459 行・テスト 27,424 行（`placement-crate-split` の段 5 `dfc9974f` の時点。`_tests.rs`・`_test_support.rs`・`test_support.rs`・`test_crash_report.rs`・`placement_tests/` の全部をテストに数えた）。起票のとき（main `e3c2cbe4`）は `crates/areka/src/placement/` の 73 ファイル・コード 7,661 行・テスト 20,167 行。
- まだ audit をかけていない。指摘はギャップ分析で洗い出す（起票のときに出さなかったのは、着手までにコードが変わって指摘が古びるため）。

## Desired Outcome

- 受け持つ範囲に `/ponytail:ponytail-audit` をかけて出た指摘が、3 つの組（必ず直す・直したほうがよい・できれば）とも全部直っている（開発者「とにかく指摘事項は全部直す方針」）。
- 直した後に同じ範囲へもう一度 audit をかけて、新しい指摘が 0 件（直しが生んだ指摘も直す）。
- 全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`）が緑。直したバグ・危ない所には、それが壊れたら落ちるテストが 1 本ずつある。

## Approach

`/kiro-start` の各段で、次のように進める（2026-10-19 開発者「原則、ギャップ分析でauditを実施して、設計以降で修正」）。

- **要件**: 「受け持つ範囲の audit の指摘を、3 つの組とも全部直す」という形だけを書く。指摘の中身はまだ書かない。
- **ギャップ分析**: 受け持つ範囲だけを対象にして `/ponytail:ponytail-audit` をかける。指摘の一覧はギャップ分析の報告に残す。番号は audit の番号をそのまま使う。audit の作法のうち 2 つは外す。
  - 「最大 20 件」は外す。全件を出し、省いた件を残さない。
  - 「読まなかった所（Not checked）」は残さない。範囲のファイルは全部読む。
- **要件の討議**: 指摘の一覧を要件に落として、直す対象を確定する。直さずに残す 3 種類（「Constraints」）は、ここで議題にする。
- **設計**: 指摘ごとの直し方を決める（何を消すか・どこにまとめるか・範囲の外の呼び出し元への影響）。直し方は ponytail の「いちばん小さな完全な変更」に従う。消せるものは消し、呼び出し元・テスト・fixture まで直し切る。
- **タスク・実装**: 直す。最後に同じ範囲へ audit をかけ直して、0 件を確かめる（検証の段のタスク）。

## Scope

- **In**: 受け持つファイルの指摘の直し・そのためのテストの追加と直し・直しのために要る範囲の外の呼び出し元の直し（下の「Constraints」の約束に従う）。
- **Out**: 範囲の外の見直し（ほかの 32 本の持ち物）・新しい機能・`vendors/`・clippy の指摘の機械的な直し（`clippy-199-lints` の持ち物。audit が同じ所を指したら、その指摘としては直してよい）。

## Boundary Candidates

- 指摘の組（必ず直す・直したほうがよい・できれば）ごと、またはファイルのまとまりごとにタスクを切る。

## Out of Boundary

- ほかの 32 本の範囲のファイル（直しが及ぶときの約束は「Constraints」）。
- 開発者の裁定（roadmap の「生きている決まり」・記憶の決めごと・完了した spec の要件）を覆す直し。直さずに議題にする。

## Upstream / Downstream

- **Upstream**: ponytail のプラグイン 5.3.2（`~/.claude/plugins`・`/ponytail:ponytail-audit`）。前に着地を待つ spec＝`placement-crate-split`（2026-10-28 `/kiro-discovery`・本体の crate 分けの 1 本目。受け持つファイルはその着地で `crates/areka-placement/src/` へ移り、同 spec がこの brief の「受け持つファイル」を書き換えた）。
- **Downstream**: なし。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `clippy-199-lints`（同じクレートに触る）。同じ範囲に触る未着手の spec は、着手のときに roadmap の spec 台帳で確かめる。

## Constraints

- **範囲の外へ直しが及ぶとき**: 関数の形を変えたら、ツリー全体で呼び出し元を探して全部直す。相手の範囲の見直しの spec が走っていれば、そのセッションと申し合わせてから触る。
- **直さずに残してよいのは次の 3 つだけ**。どれも要件の討議で開発者の裁定を仰ぐ。
  1. 開発者の裁定に反する直し。
  2. わざと置いた先送りの印。`#[allow(dead_code)]` に実在の理由の doc を付けた語彙の口（実装規律「語彙完備・配線ゼロの追跡」）と、`shortcut:`・`ponytail:` の注釈で限界が今の負荷を越えていないものがこれに当たる。
  3. 外から見た振る舞いが変わる直しのうち、SSP との互換や ukadoc の正典に関わるもの。ukadoc で確かめてから決める。
- **守る決まり**:
  - 1 ファイル 1,000 行の番人を越えない。足すなら新しいファイルへ。
  - 決定論のテストで網羅する。
  - 記録の無い失敗の道を作らない（`error!`＋`Err`）。
  - 終了の道は正規の実装にする。
  - 実機の根と一時フォルダは、ワークツリーの `target` の下に置く。
  - GPU のテストは実 GPU で回す。
  - 文字化けをコンソールの文字コードで逃げない。
  - 依存を足すなら `tech.md` へ登記して、開発者の承認を得る。
- 消す前に、ツリー全体（テスト・fixture・設定・文字列での参照を含む）で使われていないことを確かめる。
- 1 spec 20 タスクを越えそうなら、要件の討議で、指摘の組ごとに同じ範囲の 2 本目として切ってよい。

## placement-crate-split からの申し送り

`placement-crate-split`（配置を `crates/areka-placement` へ分けた spec）は配置の振る舞いを変えなかった。直したくなった所は直さず、ここへ書いた。行番号は分けた時点（段 5 `dfc9974f`）のもので、着手のときに確かめ直す。

### ⑴ 直さなかった不具合・重なり

- 分ける作業のあいだに見つけた不具合は無い。
- wintf との重なり（整理は直さずに残した）: `diag.rs` の `MonitorRecord` と wintf の `ecs/window/transition_diag.rs` の `MonitorRecord`、`resolver.rs` の `RectPx` と wintf の矩形の型。`follow/work_area.rs` の `MonitorSnapshot` が包む `Monitor` の重なりは `audit-placement-follow` が持つ。

### ⑵ 理由の文が古くなった `#[allow]` と説明

配置は bin の中のモジュールから lib の crate になり、examples も `#[path]` でソースを取り込まなくなった。次の理由の文は「areka は lib を持たない bin crate ゆえ `pub` でも dead_code を免除されない」「examples は `#[path]` で取り込む」を前提にしていて、もう事実でない。lib の `pub` の項目には lint が鳴らないので、属性そのものが要るかも確かめる。

- `#[allow]`: `lib.rs:71`（`#[allow(unused_imports)]`・理由は 69〜70 行）・`lib.rs:123`（「`#[path]` include する example」）・`lib.rs:789`（「呼び手は example とテスト」・`pub` なので属性が要らない見込み）。
- `diag.rs:53〜56` の説明（残る 2 つの `#[allow(dead_code)]`＝167 行・248 行の前置き）が「areka は bin crate」を理由にしている。167・248 行の個別の理由（語彙の口・檻専用の列）は今も事実。
- 属性ではない説明: `descript_window_keys.rs:7`・`measure_first_frame.rs:11`・`spawn.rs:35`・`spawn.rs:537`（examples の `#[path]` 取り込みのために `crate::` を使わない、という理由）、`crates/areka/src/placement_tests/spawn_assembly_tests.rs:185`（「example の `#[path]` include 不変条件」）。
- 古くなっていない `#[allow(dead_code)]`（理由が今も事実）: `descript_window_keys.rs:188`・`lib.rs:838`・`resolver.rs:418`・`source.rs:131`・`zorder_group_ledger.rs:668`。

### ⑶ 外から使うために開けた項目（2.5・3.3・3.8・段 2）

本体の bin と本体のテストが外から使うので、`pub(crate)` から `pub` にした。要らない公開が無いかを見直す。`follow*` の分は `audit-placement-follow` に書いた。すべて `crates/areka-placement/src/` の下。

- `lib.rs` のモジュール（E0603）: `balloon_limit`・`balloon_lock`・`balloon_syncscale`・`descript_window_keys`・`dpi_sync`・`face_windowposition`・`reseed`・`reset_balloon_pos`・`reset_pos`・`script_alignment`・`sticky_group_ledger`・`zorder_chain_compose`・`zorder_group_ledger`。
- `lib.rs`: `pub use windowposition::scale_signed` と定義の `windowposition.rs::scale_signed`（E0364）・`load_scope_windowpositions`・`primary_monitor`。
- 項目: `source.rs` `read_kv_lenient`・`balloon_limit.rs` `apply_balloon_limit`・`balloon_lock.rs` `BalloonLockLedger`・`seed_balloon_lock`・`replace_balloon_lock_descript`・`apply_script_balloon_lock`・`balloon_syncscale.rs` `register`・`dpi_sync.rs` `DpiSyncHold`・`HoldSite`・`current_frame`・`apply_dpi_phase_gate`・`defers_window_write`・`face_windowposition.rs` `apply_face_windowposition`・`reseed.rs` `BalloonPlacementInputs`・`apply_shell_descript`・`reanchor_char_windows`・`reset_pos.rs` のフィールド `DefaultPlacementBasis.source`・`.balloon`（E0451）。
- 段 2 で素の `pub` にした本番の項目（本体のテストが使う）: `dpi_sync.rs` `DPI_SYNC_HOLD_MAX_FRAMES`・`DpiSyncDecision`・`evaluate`・`DpiSyncOutcome` とフィールド（`decision`・`window_dpi`・`table_dpi`・`since_frame`）・`DpiSyncHold.since_frame`、`balloon_limit.rs` `BALLOON_LIMIT_CLAMP_TAG`、`balloon_lock.rs` `HoldReason`・`BalloonLockLedger::reason`。
- `cfg(any(test, feature = "test-support"))` で開けた支え: `lib.rs` `pub mod test_support`（`WA`・`with_com_initialized`・`synth_declared_dpi_ghost`・`EMILY_SHAPED_GHOST_DESCRIPT`・`EMILY_SHAPED_SHELL_DESCRIPT`・`emily_shaped_scopes`）・`spawn.rs` `pub mod test_support`（`two_scope_placements`・`titles`・`ghost_window_entities`。`fake_window_handle` は `cfg(test)`）・`reseed.rs` `pub mod test_support`（`booted_world`・`new_shell`・`char_positions` が `pub`・補助は `pub(crate)`）・`source.rs` `GhostTitles::from_scope_titles`・`persist.rs` `project_restore`。
- `cfg(feature = "test-support")` だけで開けた支え（3.8）: `measure.rs` `pub mod test_support`（`measure_with_reader`・`measure_baked` への入口）。

### ⑷ 依存の向きの規約が名指さない参照と輪（7.7）

`lib.rs` の頭の規約が名指すのは `resolver`・`config`・`measure`・`spawn`・`follow`・`diag` の 6 つだけで、次の参照は規約の外として変えなかった。

- 根を引く子: `reset_pos` → 根の `primary_monitor`、`follow/offset_space` → 根の `scale_signed`（後者のモジュールは `audit-placement-follow` の持ち物。根の側の形をこちらで決める）。
- 名指さない 21 モジュールの間の輪。
- 名指す 6 つのうち、名指さない所を引くもの（分けた時点の数え方＝本番のコードと機能で立つ子の `use (super|crate)::`）: `measure` → `error`・`descript_window_keys`、`spawn` → `chain_finalize`・`source`、`follow` → `balloon_limit`・`balloon_lock`・`dpi_sync`・`persist`・`transition_diag`・`chain_finalize`・`sticky_group_ledger`・`windowposition`・根の `scale_signed`。説明のリンクだけの参照: `diag.rs:112` → `super::spawn::wire_zorder_pair`・`follow/offset_space.rs:251` → `crate::measure::MeasureScaling`。

### ⑸ examples の古い説明

- この brief の持ち物の `crates/areka-placement/examples/window-placement.rs` には古いパスの説明は残っていない。
- 本体の `crates/areka/examples/collision-probe.rs:90` の `../pilot/...` の記述は古い（`emo2_root` は今は sample_ghost_kit から来る）。持ち物は `audit-areka-app` で、そちらへも申し送る（`placement-crate-split` の完了の棚卸で起票）。

## 2026-11-13 棚卸㊱の再測定（main `069f1a48`）

- **前提の変化**: 待つ相手の `placement-crate-split` は 11-06 に着地した（PR#433）。その後に `crates/areka-placement/` と `crates/areka/src/placement_tests/` を触った spec は無いので、受け持つファイルと申し送り ⑴〜⑷ の行の番号は今も合う（`lib.rs` 71・123・789・838、`diag.rs` 53〜56・167・248、`spawn.rs` 35・537 ほかを確かめた）。同 spec は振る舞いを変えていない。ここの指摘を先に直したものも無い。
- **触るファイル**: 114 ファイル（`crates/areka-placement/` の 105〔`Cargo.toml`・`examples/` 込み〕と `placement_tests/` の 9〔`mod.rs` 込み〕）。1,000 行の近く: `measure_tests.rs` 999・`persist_restore_tests.rs` 939・`persist.rs` 928・`lib.rs` 918。
- **C19 の約束とのずれ**: roadmap の C19 には「見直しの 5 本は `crates/areka/` に触らない」という約束がある。ところが、この spec の受け持ちには `crates/areka/src/placement_tests/` の 9 ファイルが入っていて、ぶつかる。そのうち 7 ファイル（`measure_bake_count_tests`・`measure_bake_log_tests`・`measure_template_tests`・`reset_balloon_pos_tests`・`spawn_zorder_pair_deferred_tests`・`zorder_group_branch_coverage_tests`・`zorder_inversion_branch_coverage_tests`）は、同じウェーブの `runtime-module-untangle` が `crate::emo2_boot::` の改名で必ず書き換える。次の `runtime-crate-split` は、それを `areka-runtime` へ移す。案: C19 では、このフォルダの指摘は洗い出すだけにする。直すのは `runtime-crate-split` の着地の後へ回す。また、`zorder_*_branch_coverage_tests` は `zorder_group_ledger*.rs`・`zorder_chain_compose*.rs` を名前で読む（`crate_source!`）。そのため、この 4 ファイルの名前を変えたり分けたりすると、本体の側にも響く。
- **公開の形**: この crate を引くのは本体（`emo2_boot` 93 ファイル・`input_events` 31・`menu` 8 ほか）と wintf の 2 か所。wintf の `ecs/window_proc/lifecycle.rs` 34 行目は `diag::DESPAWNED_SKIP_TAG` の綴りを、`window_pos_tests.rs` 14 行目はログの出所 `areka_placement::diag` を名指す。申し送り ⑶ の「要らない公開を閉じる」で本体の呼び出し元を書き換えると、`crates/areka/` に触ってしまう。C19 のあいだは公開の形を変えず、足すだけにする（C18 の約束 ⑴ と同じ形）。
- **`audit-placement-follow` との境目**: 受け持つファイルは重ならない。ただ、参照は両向きに多い。`follow*`・`transition_*` の 36 ファイルが、こちらのモジュール（`balloon_limit`・`balloon_lock`・`dpi_sync`・`persist`・`chain_finalize`・`sticky_group_ledger`・`windowposition`・`test_support` ほか）を引く。こちらの 32 ファイルは `follow`・`transition_diag` を引く。また、こちらの `placement_tests/mod.rs` は、相手の `transition_judge`・`transition_judge_offset` を宣言している。同じウェーブに置くなら、相手のファイルが使う項目の署名を相談なしに変えない（足すだけ）。⑷ の根の `scale_signed` の形を変えるときも、相手と申し合わせる。
- **規模**: 指摘の数で決まる。範囲は見直しの中で一番大きい（コード 10,459 行・テスト 27,424 行）ので、20 を越える見込みが高い。越えたら、要件の討議で指摘の組ごとに 2 本目を切る（Constraints のとおり）。
- **先に要るもの**: なし。
- **優先度の区分**: A（開発者「とにかく指摘事項は全部直す方針です」）。
- **要件定義のモデル**: Opus（今ある crate の中の直しで、新しい境目は作らない）。
- **分割の案**: 今は無し（指摘を見てから決める）。
- **古くなった記述**: ⑸ の「`crates/areka/examples/collision-probe.rs:90` の `../pilot/...` の記述は古い」は、もう当たらない（`placement-crate-split` が直し、今は `SampleRoot::acquire("emo2")` の説明になっている）。`audit-areka-app` の brief にもこの件は載っていない。
