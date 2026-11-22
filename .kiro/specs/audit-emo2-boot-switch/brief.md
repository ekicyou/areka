# Brief: audit-emo2-boot-switch

> 2026-10-19 `/kiro-discovery` で起票。開発者「今相談した、ポニーテールによるコードの見直しを、なるべく並行作業が出来るように複数スペックに起票して欲しい。とにかく指摘事項は全部直す方針です。」。同じ日の問いへの答え＝指摘を洗い出すのは各 spec の着手のとき・版の段は「今やる！最優先！」（同じ日のうちに改め＝v0.1.0 の直後の版 v0.1.1 へ移した・開発者「えーっと、ponytailをすぐかけたい！って言ってましたが、0.1.0（ぱすたが動くまで）の直後、0.1.1フェーズに変更してください。」・2026-10-22 さらに改め＝版の段から外し、v0.1.0 の後に隙間で取る優先度が高い列へ移した＝開発者「audit系スペックのロードマップ優先度を、Ver0.1.0が終わった後の、優先度が高いいつでもやれる項目とせよ。基本的に0.1.0が終わった後、隙間で実施したい。」・v0.2.0 の作り直しと重なる分は作り直しの前に済ませる＝roadmap の「版の段」が正本）・範囲は本番のクレート全部と開発の道具のクレートと `tools/` の台本と `pilot`。全 33 本のうちの 1 本で、分け方と共通の約束は roadmap の「テーマ別の決めごと」の「ponytail による見直し」が正本。行数は起票のとき（main `e3c2cbe4`）の実測＝着手のときに測り直す。

## Problem

開発者は、areka のコードに残っているバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードを、一度まとめて直したい。今までは、各 spec が自分の触る所だけを見てきた。全体を同じ物差しで見直したことはない。物差しは ponytail のプラグイン（5.3.2・利用者全体に入れてある）の `/ponytail:ponytail-audit`。この spec は **areka 本体の emo2_boot のうち切り替え・バルーンの見え方・ホバー・移動の cue** を受け持つ。

## Current State

- 受け持つファイル（2026-11-17 `runtime-crate-split` で書き換えた＝前は 2026-11-13 `runtime-module-untangle` の `crates/areka/src/runtime/` の `switch_assets*`・`balloon_visibility*`・`hover_inject*`・`move_cue*`・`change_cue*`・`skin_switch.rs`・`balloon_hover.rs`・`shell_box_hover.rs`。名前の型を `crates/areka-runtime/` と `crates/areka/src/runtime_tests/` の両方に当てる。新旧は roadmap の「本体の crate 分け」の新旧のパスの対応表）:
  - `crates/areka-session/src/` の `ghost_switch*`・`shell_balloon_switch*`、および `crates/areka/src/session_tests/` の `shell_balloon_switch*`。
  - `crates/areka-runtime/src/` の `switch_assets*`・`balloon_visibility*`・`hover_inject*`・`move_cue*`・`change_cue*`・`skin_switch.rs`・`balloon_hover.rs`・`shell_box_hover.rs`、および `crates/areka/src/runtime_tests/` の `balloon_visibility*`（代わりの親 2 と、本体の項目を引くテストの 5 本）。
- 大きさ: 50 ファイル・コード 5,672 行・テスト 12,687 行（`_tests.rs`・`tests/`・`_test_support.rs` をテストに数えた）。
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

- **Upstream**: ponytail のプラグイン 5.3.2（`~/.claude/plugins`・`/ponytail:ponytail-audit`）。前に着地を待つ spec＝`mcp-reload`（v0.1.0・2026-10-19 に着手中）。同 spec が `ghost_switch.rs`・`shell_balloon_switch.rs` に触るので、その着地の後に着手する（着手のときに main を取り込んでから audit をかける）。2026-10-28 `/kiro-discovery` で足した待ち＝`runtime-crate-split`（本体の crate 分けの 3 本目）。受け持つファイルのうち `ghost_switch*`・`shell_balloon_switch*` は `runtime-module-untangle` で `crates/areka/src/session/` へ、残り（`switch_assets*`・`balloon_visibility*`・`hover_inject*`・`move_cue*`）は `crates/areka/src/runtime/` を経て `crates/areka-runtime/src/` へ移る。両 spec がこの brief の「受け持つファイル」を書き換える。
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

- **要らない項目: `crates/areka/src/session/shell_balloon_switch.rs` 37〜40 行目の出し直しが設計より広い**。設計の 5 型（`SkinKind`・`SkinOrigin`・`SkinRequest`・`SkinSpec`・`SkinVerdict`）に加えて `SkinRequestRaw`・`SkinSwitchInFlight`・`SkinSwitchStage`・`skin_ref_name`・`skin_ref_path` を出し直している（テストが入口のパスで引くため）。引く所を `crate::runtime::skin_switch` へ直して、出し直しを 5 型へ絞る。

## runtime-crate-split からの申し送り（2026-11-17）

`runtime-crate-split`（`runtime` を crate `areka-runtime` へ分けた spec）は振る舞いを変えなかった。作業中に直したくなった所は直さず、ここへ書いた。行番号は 2026-11-17 時点のもので、着手のときに確かめ直す。受け持つファイルの置き場は、上の「Current State」のとおり `crates/areka-runtime/` と `crates/areka/src/runtime_tests/` の両方。

- **⑴ 直さなかった不具合**: 0 件（`runtime-crate-split` は作業中に不具合を見つけていない）。

- **⑵ 理由の文が古くなった `#[allow(...)]`**: この spec の持ち物（`switch_assets*`・`balloon_visibility*`・`hover_inject*`・`move_cue*`・`change_cue*`・`skin_switch.rs`・`balloon_hover.rs`・`shell_box_hover.rs`）に、理由が「`areka` は lib target を持たない bin クレート」ほかで移した後に事実でなくなった `#[allow(...)]` は 0 件（`rg -n "#!?\[allow\(" crates/areka-runtime crates/areka/src/runtime_tests` の全部を読んだ。当たるのは `frame.rs`・`frame/wiring.rs` の分が `audit-emo2-boot-frame`、`assets.rs` の 1 本と `transition_judge.rs` の説明が `audit-emo2-boot-spine`・`audit-placement-follow-residue`）。

- **⑶ `runtime-crate-split` で私有から開けた項目**（この spec の持ち物の分。全体の一覧は `.kiro/specs/completed/runtime-crate-split/verification.md` の 2.5・4.2・6.1）:
  - 2.5（`pub(crate)` → `pub`・全体 112 件）のうち 26 件: `balloon_hover.rs` 3・`change_cue.rs` 3・`shell_box_hover.rs` 5・`skin_switch.rs` 11・`switch_assets.rs` 4。`pub` の項目が名指す型には「要件 2.5: 公開の `<名前>` が名指すので開ける」の 1 行が付いている（`SwapFinish`・`SwapBuilt`・`SwitchBuildError` ほか）。使われない `pub(crate)` は消していない。
  - 4.2 ⑴（本体のテストが使うので `pub(crate)` → `pub`）: `BalloonHover::is_balloon_hovered`・`balloon_visibility` の `decide`／`configured_timeout_secs` の再公開と定義・`VisibilityTrigger`・`SuppressionKinds`（`dragging`・`hover`）・`MeasurementDiscardReason`・`VisibilityLogEvent`・`MeasurementOrigin`・`VisibilityAction`・`VisibilityDecision`（`actions`・`logs`・`timeout_notice`）・`ScopeObservation`（`visible_glyphs`・`visible`・`hover`・`box_showing`）・`ScopeVisibility`・`GlyphObservation`・`VisibilityObservations`（`scopes`・`lifecycle`・`dragging`）・`BalloonVisibilityState`（`forget_scope`）・`balloon_visibility::test_support`（`any(test, feature)`）と中の関数と `Frame`・`run_balloon_visibility_phase`・`balloon_visibility_phase::test_support` と `attach_headless`・`settle_box_hover`。
  - 4.2 ⑵ 家の型 `test_open`（中で tracing を呼ばない項目）: `balloon_visibility_phase.rs` の `IssueOutcome`（欄も `pub`）・`issue_actions`・`clear_hover_residency`。
  - 4.2 ⑶ 宣言の印の切り替え: `balloon_visibility::phase`。
  - 4.2 ⑷ 渡し口（`#[cfg(feature = "test-support")]` の `pub fn`・開けた私有の項目 7 に渡し口 7）: `balloon_visibility_phase` の `collect_observations`・`observe_scope_boxes`・`notify_timeout`・`emit_visibility_logs`（同名 4・`balloon_visibility_phase_test_support.rs`）・`BalloonVisibilityState` の欄 `per_scope`（`&mut`）・`display_end`・`observation_failure_logged.runtime`（`per_scope`・`display_end`・`observation_failure_logged_runtime`）。
  - 4.2 の案 C（テストを `#[test]` 関数の単位で割った）のうちこの spec の分: `balloon_visibility_phase_tests.rs`（1 戻し〔`dragging_requires_both_balloon_window_and_drag_marker`〕・14 残り）・`balloon_visibility_phase_box_tests.rs`（6 戻し・4 残り）。両方が使う補助 `runtime`・`world_at` は `balloon_visibility_phase_test_support.rs`（既存）にある。**本体の `runtime_tests/balloon_visibility_phase_tests.rs` の `world_at`（146 行）は、この支えの `world_at` と同じ中身の前からの写し**なので、支えの側の 1 つにまとめられる。
  - 6.1 examples: 持ち物の分に関わる変更は 0 件。

## audit-emo2-boot-spine からの申し送り（2026-11-17）

`audit-emo2-boot-spine` のギャップ分析（`.kiro/specs/completed/audit-emo2-boot-spine/research.md` の 6 節）で、範囲の外のこの spec の持ち場に見つけた件。中身は向こうでは直さない。

- **ウ（件 6 の 2 本）**: `crates/areka/src/runtime_tests/balloon_visibility_phase_tests.rs:L714`・`L789` のテスト関数 2 本は、補助ごと `areka-runtime` へ戻せる（`world_at` の写しは上の runtime-crate-split の申し送りに既にある）。
- **オ（`code_only` の写し）**: `crates/areka-runtime/src/balloon_visibility_phase_zorder_chain_tests.rs:L257` の `code_only` は、`source_text_test_support::code_only` と 1 字も違わない。そちらを使う形に寄せる。

## audit-emo2-boot-frame からの申し送り（2026-11-21）

`audit-emo2-boot-frame` の範囲の外で見つけた、この spec の持ち物の件（`.kiro/specs/completed/audit-emo2-boot-frame/research.md` の 6 節キ・ク）。

- **キ**: `crates/areka-runtime/src/move_cue_apply_move_tests.rs`・`move_cue_place_tests.rs` に、偽の HWND の輪（`areka_placement::test_support::give_fake_handles`）の写しがある（30 番と同じ形）。写しをやめて共通のものを使う。`move_cue_sink_reaches_runtime_wiring_receiver` の置き場の説明が誤っている件（63 番）は、向こうの直しで済ませるので渡さない。
- **ク**: `crates/areka-runtime/src/balloon_visibility_phase.rs` の `marker.scope as u32` は、`audit-emo2-boot-frame` で置いた窓の scope の上限の関数（`crates/areka-runtime/src/target_map.rs` の `window_scope`）を使う形に直す（`i32::MAX` を越える scope を読み飛ばす・`warn!` を残す）。
- **コ（`areka-runtime` の分）**: `crates/areka-runtime/src/move_cue_apply_move_sticky_tests.rs` の 256 行と `move_cue_place_tests.rs` の 395 行の `let _ = parts.handle.join();` が、記憶の書き手の panic を捨てている。`audit-emo2-boot-frame` の 20 番と同じ型で、`areka_runtime::spine::join_bounded` で待って panic を捨てない形に直す。

## 2026-11-21 棚卸㊶の再測定（main `31ac3eaf`）
- **前提の変化**: `audit-emo2-boot-frame`（PR#484）が上の申し送りのキ・ク・コを足した。どれも範囲の中のファイル（`move_cue_apply_move_tests.rs`・`move_cue_place_tests.rs`・`move_cue_apply_move_sticky_tests.rs`・`balloon_visibility_phase.rs`）で、範囲は広がっていない。クが使う `target_map::window_scope`（`crates/areka-runtime/src/target_map.rs:48`）とコが使う `spine::join_bounded` は呼ぶだけで、そのファイルには触らない。`input-crate-split`（PR#486）は範囲に触っていない（本体の `runtime_tests/balloon_visibility*` は今も `crate::input_events::…` を引く。書き換えは `session-module-gather`）。PR#488 はコメントだけで、行が動いた。
- **残る件**（行は今の main）:
  - `session/shell_balloon_switch.rs:36〜39` の出し直しが 10 項目。
  - ウ: 本体の `runtime_tests/balloon_visibility_phase_tests.rs` の 2 本と、146 行の `world_at` の写し。
  - 件 10 とキ: `move_cue_apply_move_tests.rs:44` の `attach_fake_handles`（`areka_placement::test_support::give_fake_handles` の写し）と `move_cue_place_tests.rs:82〜90` の偽の HWND、`SharedFakeIo`（`move_cue_apply_move_sticky_tests.rs:200`・`move_cue_place_tests.rs:321`）。
  - ク: `balloon_visibility_phase.rs:459` の `marker.scope as u32`。コ: `move_cue_apply_move_sticky_tests.rs:255`・`move_cue_place_tests.rs:386`。
- **ほかの brief から指される件**: `audit-areka-app` の申し送りシが、この範囲の `session/ghost_switch_tests.rs:84` の `boot_context` の写しを挙げる。この spec が直す（同じ範囲の `ghost_switch_test_support.rs` へ寄せる）形にしないと、C26 で `audit-areka-app` と同じファイルに触る（`audit-areka-app` の棚卸㊶の節）。
- **触るファイル**: 範囲は Current State のまま。上限に近いのは `session/ghost_switch_tests.rs` 982・`balloon_visibility_tests.rs` 960・`session/ghost_switch.rs` 926。
- **規模**: 指摘の数で決まる（20 を越えたら指摘の組で同じ範囲の 2 本目）。
- **先に要るもの**: `session-crate-split` と、その前の `session-module-gather`。`session-crate-split` は `ghost_switch*`・`shell_balloon_switch*` を `crates/areka-session/src/` へ移し、C3・C4 の呼び所（`ghost_switch.rs:502〜523・673`・`shell_balloon_switch.rs:131`）を書き換える。`session-module-gather` は本体の `runtime_tests/balloon_visibility*` 5 本のパスを直す。`areka-runtime` の側だけなら crate 分けに触られないが、範囲の半分だけ先に始めるのは分割になる＝先には始めない。見込み C26。
- **待たれている**: `balloon-script-offset-align`（`move_cue*`）はこの spec の後。
- **同じウェーブに置けない相手**: crate 分けの 2 本・`balloon-script-offset-align`・`doc-compat-architecture-rename`（`move_cue_geometry.rs` が文書を名指す）。`audit-areka-app`・`audit-areka-install-update` とは、上のシを持ち主が直せば 0。
- **優先度の区分**: A（開発者「とにかく指摘事項は全部直す方針です」）。要件定義のモデル: Opus（見直しの型どおり）。分割の案: 無し。
