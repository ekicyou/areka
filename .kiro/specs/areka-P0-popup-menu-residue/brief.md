# Brief: areka-P0-popup-menu-residue

起票: 2026-09-19（`areka-P0-popup-menu-minimal` の最終検証が「重大ではないが引受先が無い」と数えた残件の受け皿・開発者指示「どこにも分類されていない問題は起票」）

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
