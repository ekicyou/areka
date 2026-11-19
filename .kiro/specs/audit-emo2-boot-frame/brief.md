# Brief: audit-emo2-boot-frame

> 2026-10-19 `/kiro-discovery` で起票。開発者「今相談した、ポニーテールによるコードの見直しを、なるべく並行作業が出来るように複数スペックに起票して欲しい。とにかく指摘事項は全部直す方針です。」。同じ日の問いへの答え＝指摘を洗い出すのは各 spec の着手のとき・版の段は「今やる！最優先！」（同じ日のうちに改め＝v0.1.0 の直後の版 v0.1.1 へ移した・開発者「えーっと、ponytailをすぐかけたい！って言ってましたが、0.1.0（ぱすたが動くまで）の直後、0.1.1フェーズに変更してください。」・2026-10-22 さらに改め＝版の段から外し、v0.1.0 の後に隙間で取る優先度が高い列へ移した＝開発者「audit系スペックのロードマップ優先度を、Ver0.1.0が終わった後の、優先度が高いいつでもやれる項目とせよ。基本的に0.1.0が終わった後、隙間で実施したい。」・v0.2.0 の作り直しと重なる分は作り直しの前に済ませる＝roadmap の「版の段」が正本）・範囲は本番のクレート全部と開発の道具のクレートと `tools/` の台本と `pilot`。全 33 本のうちの 1 本で、分け方と共通の約束は roadmap の「テーマ別の決めごと」の「ponytail による見直し」が正本。行数は起票のとき（main `e3c2cbe4`）の実測＝着手のときに測り直す。

## Problem

開発者は、areka のコードに残っているバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードを、一度まとめて直したい。今までは、各 spec が自分の触る所だけを見てきた。全体を同じ物差しで見直したことはない。物差しは ponytail のプラグイン（5.3.2・利用者全体に入れてある）の `/ponytail:ponytail-audit`。この spec は **areka 本体の emo2_boot のうち毎フレームの組み立て** を受け持つ。

## Current State

- 受け持つファイル（2026-11-17 `runtime-crate-split` で書き換えた＝前は `crates/areka/src/runtime/frame/` と `runtime/frame_*.rs`。名前の型を `crates/areka-runtime/` と `crates/areka/src/runtime_tests/` の両方に当てる）:
  - `crates/areka-runtime/src/frame/`・`frame.rs`・`frame_*.rs`（`frame_test_support.rs`・`frame_shared_test_support.rs`・`frame_balloon_runtime_test_support.rs` などの支えを含む）。
  - `crates/areka/src/runtime_tests/frame/`・`frame.rs`（代わりの親）・`frame_*.rs`（本体の項目を引くテストの置き場）。
  - 終わる相の `crates/areka/src/session/ghost_quit.rs`・`ghost_quit_tests.rs`・`ghost_quit_logsink_tests.rs`・`ghost_quit_switch_tests.rs`（2026-11-13 `runtime-module-untangle` で書き換えた＝終わる相は前の `frame.rs` と `frame_ghost_quit_*_tests.rs` から移した。新旧は roadmap の「本体の crate 分け」の新旧のパスの対応表）。
- 大きさ: 60 ファイル・コード 4,845 行・テスト 21,175 行（`_tests.rs`・`tests/`・`_test_support.rs` をテストに数えた）。
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

- **Upstream**: ponytail のプラグイン 5.3.2（`~/.claude/plugins`・`/ponytail:ponytail-audit`）。前に着地を待つ spec＝`runtime-crate-split`（2026-10-28 `/kiro-discovery`・本体の crate 分けの 3 本目。受け持つファイルは `runtime-module-untangle` で `crates/areka/src/runtime/`、`runtime-crate-split` で `crates/areka-runtime/src/` へ移り、型の名前も `Emo2Wiring` → `RuntimeWiring` ほかに改まる。両 spec がこの brief の「受け持つファイル」を書き換える）。
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

## runtime-module-untangle からの申し送り（2026-11-13）

`runtime-module-untangle`（`emo2_boot` を `runtime` と `session` へ分けた spec）は振る舞いを変えなかった。作業中に直したくなった所は直さず、ここへ書いた。行番号は 2026-11-13（`93aca049f`）のもので、着手のときに確かめ直す。

- **要らない項目: `crates/areka/src/runtime/frame/wiring.rs` 322 行目の説明の文の古いパス**。`super::super::input_events::balloon` は `runtime` の下を指すが、`input_events` は本体の根（`crate::input_events::balloon`）にある。字面だけの直し。

## audit-wintf-window からの申し送り（2026-11-14）

`audit-wintf-window`（wintf の窓と窓の手続きの見直し）の完了の棚卸で、この spec の範囲で見つけたもの。直し方は決めない。場所は `audit-wintf-window` の着地の時点の行。

- **件 1**: `crates/areka/src/runtime/frame_transition_atomicity_tests.rs:47`・`:527` が `crates/wintf/src/ecs/window_proc/window_pos.rs:464`・`:468-489` を行番号で指すが、今の `guarded_set_window_pos` の呼び出しは 475 行目あたり（前から外れていた）。名前で引く形にするとずれない。
- **件 2**: `crates/areka/src/runtime/frame/zorder_drain.rs:52`・`zorder_descript.rs:33`・`zorder_drain_tests.rs:443` が、記録の出力先を 1 つに揃える理由を「grep 対象が 2 本に割れる」と書く。読み手の絞り込み（`RUST_LOG`）は前方一致なので漏れはしない。`audit-wintf-window` は同じ種類の文を「出力先は 1 つのモジュールに揃える（絞り込みは前方一致なので漏れはしないが、記録の関数を 1 つのファイルに集めて固定する）」に直した（指摘 34・R2-1）。

## runtime-crate-split からの申し送り（2026-11-17）

`runtime-crate-split`（`runtime` を crate `areka-runtime` へ分けた spec）は振る舞いを変えなかった。作業中に直したくなった所は直さず、ここへ書いた。行番号は 2026-11-17 時点のもので、着手のときに確かめ直す。受け持つファイルの置き場は、上の「Current State」のとおり `crates/areka-runtime/` と `crates/areka/src/runtime_tests/` の両方。

- **⑴ 直さなかった不具合**: 0 件（`runtime-crate-split` は作業中に不具合を見つけていない）。

- **⑵ 理由の文が古くなった `#[allow(...)]`**（属性と文は変えていない。理由が「`areka` は lib target を持たない bin クレート」で、`areka-runtime` は lib なので事実でなくなったもの）:
  - `crates/areka-runtime/src/frame.rs` 96〜97 行の文と、それが理由の `#[allow(unused_imports)]` 17 本（98〜134 行）。
  - 同 152 行の文と、それが理由の `#[allow(unused_imports)]`（153 行）。
  - `crates/areka-runtime/src/frame/wiring.rs` 12 行の文と、それが理由の `#[allow(unused_imports)]`（13 行）。
  - 次の 3 本（`frame.rs` の 171・178・188 行）の文は bin を理由にしていないが、「非 test ビルドで未使用」の名の一覧が lib の作りでも同じか、`cargo rustc -p areka-runtime --lib -- --force-warn unused_imports` で確かめていない。
  - 直し方は決めない。本体の `structure.md` の「ファサード形式も可」の注にある確かめ方（`--force-warn unused_imports`）で、まだ要る許可だけを残す。

- **⑶ `runtime-crate-split` で私有から開けた項目**（この spec の持ち物の分。全体の一覧は `.kiro/specs/completed/runtime-crate-split/verification.md` の 2.5・4.2・6.1）:
  - 2.5（`pub(crate)` → `pub`・全体 112 件）のうち: `frame.rs` 2・`frame/window_state.rs` 2・`frame/wiring.rs` 5・`frame/zorder_drain.rs` 1 の 10 件。コンパイラが外から使うと言った項目だけを開け、公開の項目が名指す型には「要件 2.5: 公開の `<名前>` が名指すので開ける」の 1 行を付けた。使われない `pub(crate)` は消していない（`audit` で見直す）。
  - 4.2 ⑴（本体のテストが使うので `pub(crate)` → `pub`）: `frame::status_report`（モジュール）・`BalloonStatusLedger`・`StageObservation`・`BalloonObservation`・`run_status_report_phase`・`report_stages`・`report_balloons`・`StayOnTopState`・`run_window_state_phase`・`run_move_drain_with`（`any(test, feature)`）・`ScaleEventSeam`・`RuntimeWiring` の欄 11 とメソッド 4（`drain_lifecycle_signals` ほか）。
  - 4.2 ⑵ 家の型 `test_open`（中で tracing を呼ばない項目）: `frame/switch.rs` の `run_switch_phase`・`split`、`frame/drain_resnap.rs` の `realign_chain_once_with_source`、`frame/work_area_sync.rs` の `resnap_for_work_area_change`。
  - 4.2 ⑶ 宣言の印の切り替え: `frame::{drain_resnap, scale_notice, switch, wiring, window_state, work_area_sync}`。
  - 4.2 ⑷ 渡し口（`#[cfg(feature = "test-support")]` の `pub fn`・開けた私有の項目 11 に渡し口 14）: `drain_resnap` の `finalize_chain_once_with`・`resnap_with`（同名 2）・`work_area_sync` の `sync_monitor_snapshot_with`（1）・`status_report` の `report_observed`（1）・`BalloonStatusLedger` の欄 `last_sent`・`last_stage`・`runtime_busy_logged`（3）・`RuntimeWiring` の欄 `zorder_ledger`・`window_state`・`last_drain`（`zorder_ledger`・`zorder_ledger_mut`・`window_state`・`window_state_mut`・`last_drain_mut` の 5）・`StayOnTopState` の欄 `on`（`on`・`on_mut` の 2）。
  - 4.2 の案 C（テストを `#[test]` 関数の単位で割った）のうちこの spec の分: `frame/status_report_tests.rs`（6 本を `areka-runtime` へ戻し、送り・届けの相の 8 本が本体に残った）・`frame/status_report_stage_tests.rs`（4 戻し・3 残り）・`frame/window_state_tests.rs`（11 戻し・5 残り＝番人の補助 `window_state_events` で本体）・`frame_move_order_tests.rs`（5 戻し・1 残り）・`frame_schedule_tests.rs`（3 戻し・1 残り）。割った後も同じ名前のテストが `areka-runtime` と本体の両方にあるファイルがある（`frame/window_state_tests.rs`・`frame/status_report_tests.rs` ほか）。
  - 6.1 examples: `crates/areka-runtime/examples/collision-probe*` へ移し、`pub` にした項目は 0 件。

- **⑷ `audit-placement-follow` の範囲の外にあって、直さなかった古い参照 3 件**（`audit-placement-follow` の完了の棚卸が申し送ったもの。今の置き場で書く）:
  - 古い「唯一の位置ライター」の説明: `crates/areka-runtime/src/frame/drain_resnap.rs` 554 行（`move_window_to` の呼び出しの上）・`crates/areka-runtime/src/frame_chain_finalize_tests.rs` 200 行（明示的な再配置の行）。今の `move_window_to` は位置の書き手の 1 つであって唯一ではない。
  - `crates/areka-runtime/src/frame_work_area_resnap_hold_tests.rs` 429 行の `#[should_panic(expected = "DpiSyncHold")]` に `#[cfg_attr(not(debug_assertions), ignore = "…")]` が無い（見張りが `debug_assertions` だけで鳴る形になった。付けると `--release` の合否が変わるので `runtime-crate-split` では付けていない）。
  - `crates/areka-runtime/src/frame/move_shift.rs` 75 行の注「当てられなかったとき（材料が無い）は口の中で warn! 済み」は、`audit-placement-follow` の 5.1（かけ直し 2 回目）で面の `move` のずれのドラッグ中・組の相手への見送りが `debug!` になったことを書いていない。

- **⑸ 配置のテストだけの公開の扱い（6.4・`areka-placement` の 3 ファイル `follow.rs`・`follow/work_area.rs`・`transition_diag.rs` だけを書いた）**:

| 項目 | 本番の使い手（crate の中） | 扱い |
|---|---|---|
| `on_balloon_drag` | `spawn.rs` | `test-support` なら `pub use`・なければ `pub(crate) use` |
| `sync_drag_writer` | `reseed`・`script_alignment`・`sticky_group_ledger` | 同上 |
| `AnchorSource` | `persist`・`reseed`・`script_alignment` | 同上 |
| `WorkAreaResolution`・`work_area_for_window_with_origin` | `follow/visibility.rs` | 同上 |
| `move_window_with_route` | `chain_realign`・`follow/move_shift.rs`・`window_move` | 同上 |
| `WORK_AREA_CLAMP_TAG` | 定義元 `work_area_clamp.rs` だけ | `#[cfg(any(test, feature = "test-support"))] pub use` の 1 行だけ |
| `MonitorSources::is_empty` | 0 件 | 定義に `#[cfg(any(test, feature = "test-support"))]`（`len` に `clippy::len_without_is_empty` の許可を同じ条件の否定で付けた） |
| `MonitorDpiEntry` | 本体のテスト（公開の欄の要素の型） | 公開のまま |
| `transition_diag.rs` の `*_ALL` 4 本・`SNAPSHOT_FIELDS` ほか欄の名の定数 5 本 | 0 件（判定器とテストだけ） | 定義に `#[cfg(any(test, feature = "test-support"))]` |
| `OFFSET_VERDICT_ALL` | `log_offset_rescale` の `debug_assert!` | 公開のまま |
| `KIND_*`・`FIELD_*` ほか行の組み立て・各発行点が使うもの | 本番が使う | 公開のまま |
| `live_work_area_bottom` | `log_char_ground` | 公開のまま（本番が使う関数） |

  `audit` で見直すとき、「公開のまま」の行は本番が使う語なので切り替えない。表の全体と緑の証跡は `.kiro/specs/completed/runtime-crate-split/verification.md` の 6.4。

## audit-emo2-boot-spine からの申し送り（2026-11-17）

`audit-emo2-boot-spine` のギャップ分析（`.kiro/specs/audit-emo2-boot-spine/research.md` の 6 節）で、範囲の外のこの spec の持ち場に見つけた件。中身は向こうでは直さない。

- **ア（1 番の片割れ）**: `crates/areka-runtime/src/frame/dpi.rs:L404-L413` で、窓の側から来る scope に `2*scope+1` を掛ける所が、大きな scope であふれる。`audit-emo2-boot-spine` は台本の側（`target_map.rs` の `2*scope`）を、上限を超える番号を `None` にして warn で捨てる形に直す。こちらも同じ上限でそろえる。
- **イ（件 6 の 1 本）**: `crates/areka/src/runtime_tests/frame/window_state_tests.rs:L144` のテスト関数は、補助を分ければ `areka-runtime` へ戻せる（dev-dependencies は既にある）。
- **オ（`code_only` の写し）**: `crates/areka-runtime/src/frame/zorder_drain_tests.rs:L465` の `code_only` は、`source_text_test_support::code_only` と 1 字も違わない。そちらを使う形に寄せる。

## audit-placement-follow-residue からの申し送り（2026-11-17）

- シェルの切り替えの置き直し（`crates/areka-placement/src/reseed.rs` の `reanchor_char_windows`）は、整合待ちの札（`DpiSyncHold`）のある窓を置き直さず `debug!`（`event = "reseed_held"`・scope）を残すようになった。札のある窓は、シェルの切り替えの直後は札が外れるまで（最大 30 フレーム）前の揃え方のまま残り、札が外れるフレームの拡大率の相（`frame/dpi.rs`）の `DpiReproject` で新しい揃え方に置かれる前提。
- この前提は配置の crate の中の檻（`reseed_tests.rs` の ⑶・`resize_window_to(.., DpiReproject)` を直に呼ぶ）でしか固めていない。札が外れるフレームに本番の相が本当にその窓を書くかを、起動のフレームの見直しのときに確かめる（論点・参照: `.kiro/specs/completed/audit-placement-follow-residue/design.md` の決め 5・ReseedHold・Revalidation Triggers の 4 つ目）。
