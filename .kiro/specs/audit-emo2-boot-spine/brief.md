# Brief: audit-emo2-boot-spine

> 2026-10-19 `/kiro-discovery` で起票。開発者「今相談した、ポニーテールによるコードの見直しを、なるべく並行作業が出来るように複数スペックに起票して欲しい。とにかく指摘事項は全部直す方針です。」。同じ日の問いへの答え＝指摘を洗い出すのは各 spec の着手のとき・版の段は「今やる！最優先！」（同じ日のうちに改め＝v0.1.0 の直後の版 v0.1.1 へ移した・開発者「えーっと、ponytailをすぐかけたい！って言ってましたが、0.1.0（ぱすたが動くまで）の直後、0.1.1フェーズに変更してください。」・2026-10-22 さらに改め＝版の段から外し、v0.1.0 の後に隙間で取る優先度が高い列へ移した＝開発者「audit系スペックのロードマップ優先度を、Ver0.1.0が終わった後の、優先度が高いいつでもやれる項目とせよ。基本的に0.1.0が終わった後、隙間で実施したい。」・v0.2.0 の作り直しと重なる分は作り直しの前に済ませる＝roadmap の「版の段」が正本）・範囲は本番のクレート全部と開発の道具のクレートと `tools/` の台本と `pilot`。全 33 本のうちの 1 本で、分け方と共通の約束は roadmap の「テーマ別の決めごと」の「ponytail による見直し」が正本。行数は起票のとき（main `e3c2cbe4`）の実測＝着手のときに測り直す。

## Problem

開発者は、areka のコードに残っているバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードを、一度まとめて直したい。今までは、各 spec が自分の触る所だけを見てきた。全体を同じ物差しで見直したことはない。物差しは ponytail のプラグイン（5.3.2・利用者全体に入れてある）の `/ponytail:ponytail-audit`。この spec は **areka 本体の emo2_boot の残り（背骨・消費者の台帳・適合の台本・資産・トークの時計ほか）** を受け持つ。

## Current State

- 受け持つファイル（2026-11-17 `runtime-crate-split` で書き換えた＝前は 2026-11-13 `runtime-module-untangle` の `crates/areka/src/runtime/` のうち、ほかの 2 本の持ち物でないもの全部。名前の型を `crates/areka-runtime/` と `crates/areka/src/runtime_tests/` の両方に当てる。新旧は roadmap の「本体の crate 分け」の新旧のパスの対応表）:
  - `crates/areka-runtime/` と `crates/areka/src/runtime_tests/` のうち、`audit-emo2-boot-frame`・`audit-emo2-boot-switch` の持ち物でないもの全部（`lib.rs`・`spine*`・`consumer_ledger*`・`adapter*`・`assets*`・`talk_*`・`session_hooks.rs`・`scope_scale.rs`・`reload_args*`・`reload_cue*`・`switch_cue*`・`shell_carry*`・`shell_balloon_resolve*`・`strict_judges*`・`crash_report_test_support.rs`・`source_text_test_support.rs`・`wire_tests.rs` ほか。本体の `runtime_tests/mod.rs`・`runtime_tests/spine.rs`（`SpineHarness` の足場）を含む）。
  - `crates/areka-runtime/` の `Cargo.toml`・`examples/`（`collision-probe`）・`src/placement_tests/` のうち `transition_*` でないもの（`mod.rs`・`measure_bake_log_tests.rs`・`reset_balloon_pos_tests.rs`・`spawn_zorder_pair_deferred_tests.rs`）。`crates/areka-runtime/src/placement_tests/transition_*`（判定器の一族）はこの spec の持ち物でなく、`audit-placement-follow-residue` の持ち物。
  - `crates/areka/src/session/` の `hooks.rs`・`reload.rs`・`reload_tests.rs`・`reload_skin_tests.rs`・`runtime_boot.rs`・`runtime_boot_tests.rs`・`switch_copy.rs`・`switch_copy_tests.rs`・`same_folder_pin.rs`・`same_folder_pin_tests.rs`。runtime から `git mv` で寄せた e2e の 2 本 `script_security_e2e_tests.rs`・`readme_script_e2e_tests.rs`（`runtime_boot_tests.rs` が `#[path]` で読む兄弟）もここに入る。
  - 前に受け持っていた install・update・readme の cue（`install_cue*`・`update_cue*`・`readme_cue*`）は、`crates/areka/src/install/`・`update/`・`readme/` へ移り、`audit-areka-install-update` の受け持ちに入った（同 brief の Upstream のとおり）＝この spec の受け持ちではない。
- 大きさ: 64 ファイル・コード 9,105 行・テスト 12,538 行（`_tests.rs`・`tests/`・`_test_support.rs` をテストに数えた）。
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

- **Upstream**: ponytail のプラグイン 5.3.2（`~/.claude/plugins`・`/ponytail:ponytail-audit`）。前に着地を待つ spec＝`mcp-reload`（v0.1.0・2026-10-19 に着手中）。同 spec が `mod.rs`・`consumer_ledger.rs` に触るので、その着地の後に着手する（着手のときに main を取り込んでから audit をかける）。2026-10-28 `/kiro-discovery` で足した待ち＝`runtime-crate-split`（本体の crate 分けの 3 本目）。受け持つファイルは `runtime-module-untangle` で `crates/areka/src/runtime/`（切替・読み直し・実行系の起動は `crates/areka/src/session/`・install・update・readme の cue は各持ち主）、`runtime-crate-split` で `crates/areka-runtime/src/` へ移る。両 spec がこの brief の「受け持つファイル」を書き換える。
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

## 追記（2026-11-06 `audit-sylphya-actor` の要件の討議で回された件）

`audit-sylphya-actor` の audit の指摘のうち、ウェーブ C18 の約束（`areka-actor` の公開の形を変えない・`crates/areka/` に触らない）のために、その spec では直せない 1 件をこの spec へ回した（2026-11-06 開発者の承認・`audit-sylphya-actor` の要件の討議 議題 3）。

- **件（audit #16）**: `areka_actor::spawn_ui` はいつも `Ok` を返すのに、戻り値は `Result<_, UiSpawnError>` で、`UiSpawnError::NotUiThread` は返されない（`crates/areka-actor/src/ui.rs`）。戻り値を `(UiSender, JoinHandle)` にしてエラー型を消し、呼び手（`emo2_boot/mod.rs` の `SpawnUi`・`crates/areka-emo-text/src/actor.rs`）を全部直す。`areka-emo-text` を受け持つ見直しの spec が走っていれば申し合わせる。

## runtime-module-untangle からの申し送り（2026-11-13）

`runtime-module-untangle`（`emo2_boot` を `runtime` と `session` へ分けた spec）は振る舞いを変えなかった。作業中に直したくなった所は直さず、ここへ書いた。行番号は 2026-11-13（`93aca049f`）のもので、着手のときに確かめ直す。

- **不具合の候補: 揺れるテスト `runtime::spine::third_scope_talk_tests::spine_third_scope_talk_runs_to_the_end_and_the_farewell_quits`**（`crates/areka/src/runtime/spine_third_scope_talk_tests.rs` 182 行目）。`cargo test -p areka -j 4` の 1 回で赤、単独 3 回と全体 1 回は緑。待ちの形（時刻か順序の前提）を調べ、テストの本質を外さずに直す。

## runtime-crate-split からの申し送り（2026-11-17）

`runtime-crate-split`（`runtime` を crate `areka-runtime` へ分けた spec）は振る舞いを変えなかった。作業中に直したくなった所は直さず、ここへ書いた。行番号は 2026-11-17 時点のもので、着手のときに確かめ直す。受け持つファイルの置き場は、上の「Current State」のとおり `crates/areka-runtime/` と `crates/areka/src/runtime_tests/` の両方。

- **⑴ 直さなかった不具合**: 0 件（`runtime-crate-split` は作業中に不具合を見つけていない）。上の 2026-11-13 の揺れるテスト（`spine_third_scope_talk_tests`）は、`runtime-crate-split` では直していない。

- **⑵ 理由の文が古くなった `#[allow(...)]`**（属性と文は変えていない）:
  - `crates/areka-runtime/src/assets.rs` 215 行の `#[allow(dead_code)] // 本番の起動はシェル名つきの兄弟を通る（呼び手はテスト）`: `build_boot_assets` は lib の `pub fn` になったので、`dead_code` の許可がまだ要るか確かめる（理由の文は今も事実）。
  - `crates/areka-runtime/src/placement_tests/transition_judge.rs` 47〜50 行の説明（`audit-placement-follow-residue` の持ち物）: 「areka は lib ターゲットを持たない bin crate ゆえ…`main.rs` が `#[cfg(test)]` の下で宣言する `placement_tests` に置いて」は、今は `areka-runtime` の `lib.rs` が宣言する。属性ではなく説明の文なので、残りの `#[allow]` の一覧と一緒に、持ち主の `audit-placement-follow-residue` へ伝える。
  - frame の持ち物の `frame.rs`・`frame/wiring.rs` の分は `audit-emo2-boot-frame` の同じ節にある。この spec の持ち物で、ほかに理由が bin を言う `#[allow]` は 0 件。

- **⑶ `runtime-crate-split` で私有から開けた項目**（この spec の持ち物の分。全体の一覧は `.kiro/specs/runtime-crate-split/verification.md` の 2.5・4.2・6.1）:
  - 2.5（`pub(crate)` → `pub`・全体 112 件）のうち 76 件: `lib.rs` 22・`reload_args.rs` 6・`reload_cue.rs` 5・`scope_scale.rs` 4・`session_hooks.rs` 1・`shell_balloon_resolve.rs` 9・`shell_box_assets.rs` 1・`shell_carry.rs` 3・`shell_cursor_assets.rs` 3・`shell_cursor_table.rs` 5・`skin_facts_copy.rs` 1・`special_name.rs` 6・`strict_judges.rs` 1・`switch_cue.rs` 2・`talk_lifecycle.rs` 4・`user_break_cue.rs` 3。`pub` の項目が名指す型 11 には「要件 2.5: 公開の `<名前>` が名指すので開ける」の 1 行が付いている（`DescriptParts`・`ScopeScaleTable`・`NotFoundReason`・`ShellBoxAssets`・`ShellCarry`・`CursorFile`・`UnresolvedReason`・`TalkTimeout` ほか）。使われない `pub(crate)` は消していない。
  - 3.2: `spine`・`spine_wait`・`spine_hold_support` の `pub(crate)` を `pub` にした。`spine::{BACKOFF_SLEEP, SPIN_WAIT, join_bounded}` の再公開・`ScriptedShioriHandle::status_calls`（`pub fn`）・`frame_test_support`・`frame_balloon_runtime_test_support` を `test-support` の下で開けた。進行状態の記録 4 つ（`RecordedStatus`・`StatusLedger`・`record_status`・`snapshot_status_calls`）は設計の外れとして `areka-runtime` の `spine.rs` の ⑵ の隣へ下ろした（本体は出し直し）。
  - 4.2 ⑴ のうち: `assets::{build_shell_assets_with_limits, build_balloon_assets_with_limits}`・`DescriptParts::ALL`・`ScopeScaleTable::set`・`percent_text`・`NO_BODY`・`LastInstalledShell`・`LastInstalledBalloon`・`ShellBoxAssets::hand_to`・`ShellCursorAssets::hand_to`・`shell_carry` の `CarriedShow`・`BindReplay`（`category`・`part`・`on`）・`CarriedBinds`（`replays`・`dropped`）・`decide`・`decide_binds`・`snapshot`・`replay`・`ArekaStrictJudges`（`canonical`）・`SwapPayload`・`SwapSlot`。`NO_BODY` は口の倒れ先で、`areka-runtime` のテストでは記録なし・本番のビルドでは記録つき。
  - 4.2 ⑷ 渡し口のうちこの spec の分: `ScriptedShioriHandle` の欄 `status_calls`（`spine::test_support`）の 1 本。開けた私有の項目は全体で 19（tracing を呼ぶ関数 8・欄 11）・渡し口は全体で 22。
  - 4.2 の案 C（テストを `#[test]` 関数の単位で割った）のうちこの spec の分: `assets_tests.rs`（13 戻し・1 残り）・`assets_shell_tests.rs`（2 戻し・1 残り）・`consumer_ledger_agreement_tests.rs`（1 戻し・4 残り）・`shell_balloon_resolve_tests.rs`（5 戻し・本体に 4 残り）・`shell_carry_tests.rs`（4 戻し・本体に 8 残り）・`wire_tests.rs`（2 戻し・1 残り）・`strict_judges_tests.rs`（4 戻し・`doc_tables` の 4 残り）・`zorder_wiring_tests.rs`（7 戻し・2 残り）。両方が使う補助は新しい支えのファイル（`consumer_ledger_test_support.rs`・`shell_balloon_resolve_test_support.rs`・`shell_carry_test_support.rs`・`source_text_test_support.rs`）へ 1 つだけ置いた。
  - **番人の crate を引く補助を使う 17 本の関数は、本体に残した**（`log-capture-kit`・`sample-ghost-kit`・`temp-path-kit` は `test-support` に置けないので、写しを作らず、その補助を使う関数も本体に残した・設計 D4。内訳は `.kiro/specs/runtime-crate-split/verification.md` の 4.6＝表の「番人の補助で本体」12 本と `switch_cue_tests.rs` の 5 本）。これらは本体の項目を引かないのに本体にある。`audit` で番人の crate の使い方を直すときは、`areka-runtime` へ戻せるか見直す。
  - 本体の `runtime_tests/balloon_visibility_phase_tests.rs` の `world_at`（146 行）は、`areka-runtime` の支え（`balloon_visibility_phase_test_support.rs` の `world_at`）と同じ中身の前からの写しである（`audit-emo2-boot-switch` の持ち物のファイルだが、支えは `areka-runtime` の側なので、両方の `audit` が見るところ）。支えの側へ 1 つにまとめられる（本体のテストは `areka_runtime::` の支えを引く）。
  - 6.1 examples: `crates/areka/examples/collision-probe*` を `crates/areka-runtime/examples/` へ移した。`#[path]` の私有の取り込みを外し、`areka_runtime::hit_region`・`areka_runtime::target_map` を公開面で読む。子ファイルの接続 7 本の `#[path]` は example のルートの規則のために残した。`pub` にした項目は 0 件。

- **ほかに一緒に伝えること（`hit_region.rs` の説明）**: `crates/areka-runtime/src/hit_region.rs` の頭の説明の「`crate::` フリー規律（この配置の決定的制約）」（45〜57 行）は、examples が `#[path]` で取り込んでいた理由（probe からの `#[path]` include）が古くなった（今の probe は crate の公開面で読む）。`runtime-crate-split` はリンクの字面だけを直し、規律の文は直していない。非テストコードが `crate::` を使ってよいかを見直し、文を事実に直す（`lib.rs` が `is_benign_boot_error` への本体の呼び出しを持たなくなったか、`#[path]` include がもう無いかも確かめる）。
