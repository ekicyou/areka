# Brief: areka-P0-popup-menu-residue

起票: 2026-09-19（`areka-P0-popup-menu-minimal` の最終検証が「重大ではないが引受先が無い」と数えた残件の受け皿・開発者指示「どこにも分類されていない問題は起票」）

## 2026-09-20 棚卸⑮の再測定

**残件 1・2 は `areka-P0-wintf-drag-state-rest-contract`へ移した。** どちらも潜在バグで、同 spec と同じ `crates/areka/src/menu/trigger.rs` を触るためである。本仕様に残るのは 3〜10 の 8 件（記録の文言と接頭辞 4・5・6・7／テストの穴 3・8／語彙 4 件の引受先 9／World 借用中の `ShellExecuteW` 10）＝いずれも α にもバグ修正にも属さないので、当面着手しない。

**実測（main `fe157df1`）**: 残件 3〜10 は全件が実在する。ただし 7 番（`crates/areka/src/emo2_boot/spine.rs` の「現在は 4 本」「4-sink 構成」）は、本文の「実際は 6 本」も古くなった——`areka-P0-balloon-break` の `no_user_break_sink` で **7 本**（`crates/areka/src/emo2_boot/mod.rs` の `sinks: vec![…]`）。

サブメニューの登記は `areka-P0-ghost-shell-balloon-switch`（「ゴースト」枠）と `areka-P0-shell-balloon-switch`（「シェル」「バルーン」枠）が行う。`baseware-root-layout` は行わない。

## Problem

`areka-P0-popup-menu-minimal`（右クリックメニューの第 1 スライス）は最終検証 GO で閉じるが、検証とレビューが挙げた「直すほどではない／範囲外」の指摘が、tasks.md の記録に書かれただけで**引受先を持っていない**。完了アーカイブへ入ると、その記録は誰も読まなくなる（完了済み spec への先送りは消化不能）。利用者から見える影響は小さいものばかりだが、1 件は条件が揃うと別の窓へ右ダブルクリックが届く誤配で、1 件は無害な場面で `error!` が出る記録の重さの誤りである。

## Current State

根拠はすべて `areka-P0-popup-menu-minimal` の tasks.md（Implementation Notes・完了記録 9.3「最終検証で足したこと」）と 2026-09-19 の最終検証の所見。

| # | 残件 | 場所 | 利用者から見える影響 |
|---|---|---|---|
| 1 | 返事待ち中に**別の窓**で預けられた右ダブルクリックを、先の窓の「抑止」の判定で送りうる（預かりの scope と要求の scope を比べていない） | `crates/areka/src/menu/trigger.rs` の `poll_once`（`decide` の結果で `send_pending_right_double_click` を呼ぶ所） | SHIORI の返事待ち（1 秒未満）の間に別の窓で 1 クリック＋押下が要る＝実質届かない。届くと、メニューを抑止していない側の窓へ右ダブルクリックの台詞が出る |
| 2 | 台本の `\![open,readme]` は、説明書のファイルが無くても `ShellExecuteW` まで行き `error!`（符号 2）を出す（メニュー側は灰色表示で守られているが、台本側は `open_from_world` が実在を見ない） | `crates/areka/src/readme.rs` の `open_from_world` | 動作は同じ（何も開かない）。記録だけが重い。`debug!`／`warn!` へ落とすか、実在を先に見るかの裁定が要る |
| 3 | 台本の入口（台本の文字列 → 汎用キャリア → `ReadmeCueSink` → `drain_readme_requests`）を端から端まで踏む決定論テストも実機観察も無い（実機確認 9.3 ⑵ はメニューの入口だけ） | `crates/areka/src/emo2_boot/readme_cue.rs`・`readme.rs` | 無し（部品ごとのテストはある）。実機の台本に `\![open,readme]` を 1 行足して 1 度見れば足りる |
| 4 | `HMENU` の組立の失敗（`CreatePopupMenu`／`AppendMenuW`）も `[menu] TrackPopupMenuEx failed` の文言で記録される（hresult は正しいが文言が不正確） | `crates/areka/src/menu/win32.rs`・`trigger.rs` の `finish` | 無し（障害調査のときに誤読する） |
| 5 | 記録の接頭辞が不揃い: `[menu]`／`[readme]` は接頭辞＋`event=`、`ReadmeCueSink:` は `event=` 無し、`actor_resources.rs` は角括弧なし | 各ファイル | 無し（ログ検索の型が 3 通り） |
| 6 | 照会の往復の失敗の文言が「終了系列（Fault）へ」のまま（照会の経路は終了系列へ入らない） | `crates/areka-kanade/src/actor.rs` の往復の関数（3 か所） | 無し（誤解を招く記録） |
| 7 | `spine.rs` のコメント「production の sink 構成は現在 4 本」が陳腐化（実際は 6 本・`ReadmeCueSink` が 6 本目） | `crates/areka/src/emo2_boot/spine.rs`（`wire_emo2_boot` の sink 構成に触れるコメント） | 無し |
| 8 | scope 1 の終了指示を kanade の端から端まで通すテストが無い（`events_tests` は `on_close` 単体・実機ログは `reason="user"` までで Ref1／Ref2 を印字しない） | `crates/areka-kanade/tests/kanade/`・`schedule/events.rs` | 無し（受け渡しは構造上素通し） |
| 9 | 語彙のみで登記した 4 件に引受先 spec が無い: `char*.popupmenu.visible`（n≧2）と `*.popupmenu.type` 3 件（台帳 `doc/ukadoc-coverage/ledger/shiori.toml` の備考「引受先: 省略した形のメニューを作り分ける仕様と、3 人目以降のキャラクター窓を作る仕様。2026-09-18 の時点でどちらも起票は 0 本なので、先に起こした側が本欄を引き取る」） | `crates/areka/src/menu/captions.rs` の `UNQUERIED_POPUPMENU_RESOURCES` | 3 体目以降のキャラクターのメニュー抑止が効かない（α は scope 0／1 のみ）。`popupmenu.type` は値 1 が指す「省略した形」の中身を正典が定めていない（どの値でも同じメニューを出している） |
| 10 | `ShellExecuteW` を World を借りたまま呼ぶ（起動の待ち時間ぶん 1 tick が止まりうる・設計で受容済み・入れ子の tick は `try_borrow_mut` の失敗で飛ばすので安全） | `readme.rs` の `drain_readme_requests`・メニューの動作 | 説明書を開く瞬間に一瞬だけ描画が止まりうる（実機では未観測） |

本 brief に**含めないもの**（引受先が別にある）: 里々の項目名の実機観察（→ `areka-P0-shell-implicit-surface` の brief 2026-09-19 追記。同 spec は 2026-09-20 に完了し `.kiro/specs/completed/areka-P0-shell-implicit-surface/brief.md` に在る）・wintf のドラッグ状態の契約（→ `areka-P0-wintf-drag-state-rest-contract`）・網羅台帳の文書の写真の撮り直し（→ `areka-P0-coverage-roadmap-refresh`）。

## Desired Outcome

- 上の 10 件それぞれが「直した」「裁定で直さないと決めた（理由付き）」「別の spec へ渡した」のどれかになっている。
- 1 と 2 は決定論テストが付く（1 は別の窓の預かりが送られないこと・2 は記録の水準）。

## Approach

台帳型の spec（先例: `areka-P0-balloon-canon-residue`・`areka-P0-emo-text-canon-residue`）。要件段階で 10 件を「直す／直さない／渡す」に仕分け、直すものだけをタスクにする。1 は 1 行（`deferred.scope == request.scope` のときだけ送る・合わない預かりの扱いは「捨てて `trace!`」か「残す」かを要件で決める）。9 は `char{n}`（n≧2）を `areka-P0-ghost-shell-balloon-switch` 以降の多キャラクター対応へ、`popupmenu.type` は「省略した形」の中身を決める裁定が先に要る（正典が定めていないので、実装しないと決めて台帳の備考を確定させる選択肢もある）。

## Scope

- **In**: 上の表の 1〜10 の仕分けと、直すと決めたものの実装・テスト・台帳の備考の追随。
- **Out**: メニューの項目の追加（サブメニューの登記は `areka-P0-baseware-root-layout`／`areka-P0-ghost-shell-balloon-switch` が行う）・オーナードロー・トレイアイコン。

## Boundary Candidates

- `menu/trigger.rs` の判定（1）
- `readme.rs` の記録の水準（2・10）
- 記録の文言と接頭辞（4・5・6・7）
- テストの穴（3・8）
- 台帳の引受先（9）

## Out of Boundary

- `areka-P0-popup-menu-minimal` の要件の改訂（閉じた spec は開け直さない）
- kanade の終了系列の挙動

## Upstream / Downstream

- **Upstream**: `areka-P0-popup-menu-minimal`（main へ入った後）
- **Downstream**: なし

## Existing Spec Touchpoints

- **Extends**: なし（`areka-P0-popup-menu-minimal` は完了アーカイブへ入る）
- **Adjacent**: `areka-P0-ghost-shell-balloon-switch`・`areka-P0-baseware-root-layout`（同じ `menu/` へサブメニューを登記する＝着手の順序に注意）・`areka-P0-alpha-release-signoff`（3 の実機観察を相乗りできる）

## Constraints

- α の必須ではない（段は α 後）。ただし 1 と 2 は小さいので、`menu/` に触る次の spec に相乗りさせてもよい。
- 規模の見立て: S。


---

## 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- 優先度 低。残件 1・2 は完了 `wintf-drag-state-rest-contract` で済み。項目 7 の sink の数は今 **11**（`emo2_boot/mod.rs`）。項目 4・5・6・9 は残っている。
- 項目 9（引受先の無い語彙 4 件＝`shiori.toml` の `char_2a.popupmenu.type`・`char_2a.popupmenu.visible`・`kero.popupmenu.type`・`sakura.popupmenu.type`。台帳の持ち主は完了 `popup-menu-minimal` のまま）の持ち主の付け替えは `coverage-roadmap-refresh` が行う。
- **本文の古い記述**: 43・48 行あたりが `ghost-shell-balloon-switch`・`baseware-root-layout` を「これから行う」と書いているが、どちらも完了済み。3 人目以降のキャラクターの窓（`char{n≧2}`）は今どの spec も持っていない（説明書の既知の制限）。
- 単独で回すより、次に `crates/areka/src/menu/` を触る spec へ相乗りするのが安い。


---

## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 規模: S（5〜7 タスク）。切る: なし。段は「その他」のまま。
- 前提の状態: 上流なし。10-02 の後に `crates/areka/src/menu/`・`readme.rs`・`emo2_boot/readme_cue.rs` を触ったコミットは 0。
- 崩れた前提／古くなった位置:
  - 項目 7: sink の数は今も **11**（`emo2_boot/mod.rs` の `wire_emo2_boot` の `sinks: vec![…]`。`shell-balloon` は足していない）。`spine.rs` のコメントは今も「現在は 4 本」「4-sink 構成」。
  - 項目 4: `menu/trigger.rs` の `finish` の「`[menu] TrackPopupMenuEx failed`」は残っている（`trigger_show_tests.rs` が同じ文言を逐語で見ている＝文言を変えるとテストも直す）。
  - 項目 6: `areka-kanade/src/actor.rs` の往復の失敗の 3 か所（「——終了系列（Fault）へ」）は残っている。
  - 項目 9: `menu/captions.rs` の `UNQUERIED_POPUPMENU_RESOURCES` は残っている。台帳の 4 行の持ち主は完了 `popup-menu-minimal` のまま＝付け替えは `coverage-roadmap-refresh`。
  - 本文の「`ghost-shell-balloon-switch` へ渡す」（3 人目以降の窓）は、渡し先が完了済みで持ち主が居ない（roadmap の覚え書き）。
- 触るファイル（並走の照合用）:
  - `crates/areka/src/menu/{trigger.rs, win32.rs, captions.rs}`・`menu/trigger_show_tests.rs`
  - `crates/areka/src/readme.rs`・`crates/areka/src/emo2_boot/readme_cue.rs`
  - `crates/areka/src/emo2_boot/spine.rs`（コメントだけ）
  - `crates/areka-kanade/src/actor.rs`（記録の文言 3 か所）
  - `crates/areka-kanade/src/actor_resources.rs`（接頭辞を揃えるなら）
  - 共有しうる相手: `menu/` を触る未完了 spec は 0。`emo2_boot/spine.rs` は C2〜C3 の `emo2_boot` を触る spec（`shell-balloon` の後継・`balloon-lifecycle-events`）が近くを触りうるがコメント 1 か所。`areka-kanade/src/actor.rs` は kanade の進行の列（`mouse-drag-events`・`balloon-lifecycle-events` は `msg.rs`・`schedule/` で別のファイル）と重ならないが、`property-query-channels` は `actor.rs` に触る。
- 議題（答えで作業が変わるものだけ）: なし（10 件の仕分けは要件の段で）。
- 見つけた穴: なし。


## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模: S（5〜7 タスク）。切る: なし。段は「その他」のまま。
- 前提の状態: 上流なし。今すぐ着手できる。
- 崩れた前提／古くなった位置: なし。C3 で `crates/areka/src/menu/`・`readme.rs`・`emo2_boot/readme_cue.rs`・`emo2_boot/mod.rs`・`crates/areka-kanade/src/actor.rs`・`actor_resources.rs` に入った変更は 0。`emo2_boot/spine.rs` は棚卸㉑の PR で `#[allow(dead_code)]` の 2 か所に理由のコメントが付いただけ。棚卸㉑の指し先を引き直した結果:
  - 項目 7: `wire_emo2_boot` の `sinks: vec![…]` は今も 11 本。`spine.rs` の 2 つのコメント（「現在は 4 本」「4-sink 構成」）は残っている。
  - 項目 4: `menu/trigger.rs` の `finish` の「`[menu] TrackPopupMenuEx failed`」は残り、`trigger_show_tests.rs` が同じ文言を 2 か所で見ている。
  - 項目 6: `areka-kanade/src/actor.rs` の SHIORI の往復の失敗の 3 か所（「——終了系列（Fault）へ」）は残っている。
  - 項目 9: `menu/captions.rs` の `UNQUERIED_POPUPMENU_RESOURCES` は残り、台帳の 4 行の持ち主は完了 `popup-menu-minimal` のまま（付け替えは `coverage-roadmap-refresh`）。
- 触るファイル（並走の照合用）: 棚卸㉑のまま＝`crates/areka/src/menu/{trigger.rs, win32.rs, captions.rs}`・`menu/trigger_show_tests.rs`・`crates/areka/src/readme.rs`・`emo2_boot/readme_cue.rs`・`emo2_boot/spine.rs`（コメントだけ）・`crates/areka-kanade/src/actor.rs`（記録の文言 3 か所）・（接頭辞を揃えるなら）`actor_resources.rs`。
- 共有しうる相手: `menu/` を触る未完了 spec は 0。`areka-kanade/src/actor.rs` は `property-query-channels` が触る。`emo2_boot/spine.rs` はコメント 1 か所なので、近くを触る spec と並べても行の重なりは起きにくい。
- 議題（答えで作業が変わるものだけ）: なし。
- 見つけた穴: なし。

## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- **前提の変化**: 項目 10（World を借りたままの `ShellExecuteW`）は `open-external-tags`（10-06）で済んだ＝開く処理は `crates/areka/src/readme/opener.rs` の専用のスレッドへ移った。項目 2 も済み（説明書が無ければ OS を呼ばずに warn で止まる。`crates/areka/src/readme_tests.rs` に檻がある）。残りは 3〜9 の 7 件:
  - 4: `crates/areka/src/menu/trigger.rs` の、メニューを出せなかったときの記録「`[menu] TrackPopupMenuEx failed`」と、それを逐語で見る `menu/trigger_show_tests.rs` の 2 か所。
  - 5: `crates/areka/src/emo2_boot/readme_cue.rs` の「`ReadmeCueSink:`」で始まる 3 行（`open-external-tags` が文を書き換えたが、接頭辞の形は同じ）。
  - 6: `crates/areka-kanade/src/actor.rs` の SHIORI の往復の失敗の 3 か所（「——終了系列（Fault）へ」）。
  - 7: `crates/areka/src/emo2_boot/spine.rs` の 2 つの注記（「現在は 4 本」「4-sink 構成」）。本番は今も 11 本（`emo2_boot/mod.rs` の `sinks: vec![…]`）。
  - 3・8: テストの穴 2 件（台本の文字列から開く処理までの端から端・scope 1 の終了の指示の端から端）。部品ごとのテストは `open-external-tags` で厚くなった。
  - 9: `crates/areka/src/menu/captions.rs` の、照会しないメニューの語彙 4 件。台帳の持ち主の付け替えは `coverage-roadmap-refresh`。
- **触るファイル**: `crates/areka/src/menu/{trigger.rs（573 行）, win32.rs, captions.rs, trigger_show_tests.rs}`・`emo2_boot/readme_cue.rs`・`emo2_boot/spine.rs`（937 行・注記だけ）・`crates/areka-kanade/src/actor.rs`（**900 行＝上限の近く**。文言だけなので増えない）・接頭辞を揃えるなら `actor_resources.rs`・新しいテスト 1〜2 本。`readme.rs` は項目 2・10 が済んだので触らない見込み。
- **規模**: S（4〜6）。
- **先に要るもの**: なし。ファイルの重なり: kanade の `actor.rs`（`mcp-kanade-tools` ほか kanade の進行の列）・`menu/captions.rs`（`extra-character-windows`）・`emo2_boot/spine.rs`（`areka-test-threads-av`・`test-wait-marker-gaps` が同じファイルの足場に触りうる）。
- **測定の仕事ではない**（重い回 0）。
- **優先度の区分**: C（バグでない持ち越し・記録の文言とテストの穴）。
- **要件定義のモデル**: Opus。
- **分割の案**: なし。
- **見つけた穴・古くなった記述**: 冒頭の「本仕様に残るのは 3〜10 の 8 件」は 7 件になった。項目 7 の注記 2 か所は、spec を待たずに直せる（即時の直し候補）。
