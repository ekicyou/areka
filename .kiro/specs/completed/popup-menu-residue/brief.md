# Brief: popup-menu-residue

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

本 brief に**含めないもの**（引受先が別にある）: 里々の項目名の実機観察（→ `areka-P0-shell-implicit-surface` の brief 2026-09-19 追記。同 spec は 2026-09-20 に完了し `.kiro/specs/completed/areka-P0-shell-implicit-surface/brief.md` に在る）・wintf のドラッグ状態の契約（→ `areka-P0-wintf-drag-state-rest-contract`）・網羅台帳の文書の写真の撮り直し（→ `coverage-roadmap-refresh`）。

## Desired Outcome

- 上の 10 件それぞれが「直した」「裁定で直さないと決めた（理由付き）」「別の spec へ渡した」のどれかになっている。
- 1 と 2 は決定論テストが付く（1 は別の窓の預かりが送られないこと・2 は記録の水準）。

## Approach

台帳型の spec（先例: `balloon-canon-residue`・`emo-text-canon-residue`）。要件段階で 10 件を「直す／直さない／渡す」に仕分け、直すものだけをタスクにする。1 は 1 行（`deferred.scope == request.scope` のときだけ送る・合わない預かりの扱いは「捨てて `trace!`」か「残す」かを要件で決める）。9 は `char{n}`（n≧2）を `areka-P0-ghost-shell-balloon-switch` 以降の多キャラクター対応へ、`popupmenu.type` は「省略した形」の中身を決める裁定が先に要る（正典が定めていないので、実装しないと決めて台帳の備考を確定させる選択肢もある）。

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

## 2026-10-16 追記: 件 11（バグ・`areka-P0-baseware-root-list` の完了時の棚卸）

**件 11: 右クリックメニューを開いている間、うにゅう側（kero）のバルーンの窓が黒い四角で塗られて見えることがある。**

- **見つけた所**: `areka-P0-baseware-root-list` の実機確認（タスク 7.3・2026-10-16）。メニューを開いた直後に画面を撮ると、2 回とも kero のバルーンの位置が黒い四角だった（ゴースト emo2・バルーン emo2-kakukaku・debug ビルド・`TrackPopupMenuEx` の標準メニュー）。さくら側のバルーンは普通に描かれていた。記録は `.kiro/specs/completed/areka-P0-baseware-root-list/tasks.md` の Implementation Notes（7.3 で見つけた範囲外の件）。
- **まだ分かっていないこと**: メニューを閉じると元に戻るか・メニューを開いた瞬間に毎回なるか・release ビルドでもなるか・kero のバルーンが喋っている途中かどうかで変わるか。
- **見立て（未確認）**: 標準メニューのモーダルの間は UI スレッドのメッセージの回り方が変わる。その間に kero のバルーンの窓の中身（合成の面）が出し直されず、黒く見えている疑い。触る場所は `menu/trigger.rs`（メニューを開く所）と、窓の表示（wintf の合成）のどちらか。
- **まとめた理由**: 同じ右クリックメニューを開く所（`menu/`）の残件で、この spec の触る場所と重なる（「小さいバグの起票は触る場所が同じものを 1 本にまとめる」）。原因が wintf の合成の側と分かったら、要件の段でその持ち主へ渡す。
- **区分**: この件だけバグ（区分 B）。残りの件（区分 C）より先に取ってよい。まずは再現の条件を測る（実機・メニューを開く前後の画面と記録）。
- **写した先**: `popup-menu-shell-look` の brief（2026-10-31）。

## 2026-10-18 追記: 件 12（バグ・`areka-P0-emily-ghost-verification` の完了時の棚卸）

**件 12: 右クリックメニューが、何も選ばれないまま閉じる・「出した」の記録はあるのに画面に見えないことがある。**

- 見たもの: えみりの実機の 2 回目の走行（2026-10-17 23:56〜23:58・`.kiro/specs/completed/areka-P0-emily-ghost-verification/verification.md` の崩れた所 6）。メニューの「よんでね」でメモ帳を開いて閉じた後、道具のクリックで「終了」を選ぶと、メニューは何も選ばれずに閉じた。続く右クリック 3 回は `menu_shown` の記録が出るのに画面に見えず、対になる `menu_selected` も無かった。SendInput だけで出し直すと見えて、選べた。
- 見立て（未確定）: メニューの持ち主の窓が前面を取れない（メモ帳を閉じた直後・別のアプリが前に出た後）と、Win32 のメニューが見えずに閉じる。areka の窓は前面に出ない作りなので、`TrackPopupMenu` の前の前面の取り方（`SetForegroundWindow`）が効いていない疑い。
- 記録の穴: 選ばれずに閉じたときの記録 `menu_dismissed`（`crates/areka/src/menu/trigger.rs`）は DEBUG で、`info` の水準では見えずに閉じたのか、見えないまま開いているのかを区別できない。区別できるよう水準か文言を見直す。
- 再現を測るときは `RUST_LOG` に `areka::menu=debug` を足す。実機でメニューを開いて撮る仕事なので、件 11 と同じく人の待ち（入力を送る許し）になりうる。
- 優先度の区分: B（使えなくなることがある不具合）。起票の理由: 確かめるのに実機の走行を何度も回す必要がある（完了時の棚卸の起票の条件 4）。
- 写した先: `popup-menu-shell-look` の brief（2026-10-31）。

## 2026-10-24 追記: 件 13（`shiori2-protocol` のタスク 5.4・「．さくら」の右クリックメニューの資源）

**件 13: 「．さくら」が用意したメニューの資源のうち、areka が問わない名前がある。そのゴーストらしい項目名にならず、項目そのものが無いものもある。**

- 見たもの（静的・`shiori2-protocol` の研究記録 4.4 節）: 「．さくら」（華和梨・SHIORI/2.x）の辞書は資源を `resource.updatebuttoncaption`・`resource.vanishbuttoncaption`・`resource.vanishbuttonvisible`・`resource.sakura.recommendbuttoncaption`・`resource.sakura.recommendsites`・`resource.kero.recommendbuttoncaption`・`resource.kero.recommendsites`・`resource.sakura.portalbuttoncaption`・`resource.sakura.portalsites` の名前で持つ。areka が問う名前（`crates/areka-kanade/src/schedule/resources.rs` の `ALLOWED_RESOURCE_IDS`）にはどれも無い。
- 崩れ方:
  - ⑴ 古い名前: 「ネットワーク更新」の項目は areka が `updatebutton.caption` だけを問うので、「．さくら」の `updatebuttoncaption` が使われず、areka の素の言葉のままになる。ukadoc は `updatebuttoncaption` を別の項目として載せている（<https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html#updatebuttoncaption>）。
  - ⑵ 項目が無い: 消滅通告（`vanishbuttoncaption`・`vanishbuttonvisible`）・おすすめ（`*.recommendbuttoncaption`・`*.recommendsites`）・ポータル（`sakura.portalbuttoncaption`・`sakura.portalsites`）の項目を areka のメニューは持たない。
- 台帳: 件 13 の 9 つの名前は、どれも `doc/ukadoc-coverage/ledger/shiori.toml` で `vocabulary-only`・持ち主なし。
- 重なりに注意: `popup-menu-shell-look` の件 15（brief の 110 行目あたり）が、SSP がメニューを組むときに `sakura.recommendsites` を問うことを見ている。⑵ のおすすめの項目を足すときは、あちらと分け方を決める。
- 実機の 1 回（`shiori2-protocol` 5.1・2026-10-24）では右クリックメニューを開いていないので、画面では見ていない（メニューを開いたときに問う作りなので、記録にも出ない）。
- まとめた理由: 触る場所が件 9 と同じ（`menu/captions.rs` と問う名前の表）。⑵ は項目を足す仕事なので、要件の段で大きすぎると分かったら別の spec に切り出す。
- 優先度の区分: C（ukadoc の拾い残し）。直したら「．さくら」で右クリックメニューを開き、項目名を SSP と見比べる。
- 写した先: `popup-menu-extra-resources` の brief（2026-10-31・本 spec の完了時の棚卸で `/kiro-discovery` が起票）

## 2026-10-31 棚卸㉛の再測定（main 4068422f）
- **前提の変化**: 棚卸㉙（10-23）の後、件の場所のソースは動いていない（`menu/` に入ったのは `mouse-click-wheel-events` の `mod.rs` と `readonly-mode` の `install_frame.rs`・`update_frame.rs` だけで、10-24 より後は 0）。変わったのは 10-24 に件 13（「．さくら」の古い資源の名前と、無い項目）が足されたこと。済んだ件は 1・2・7・10＝残りは件 3〜6・8・9・11〜13。
- **件ごとの今**: 件 4＝`crates/areka/src/menu/trigger.rs` 487 行と `menu/trigger_show_tests.rs` 263・439 行。件 5＝`crates/areka/src/emo2_boot/readme_cue.rs` の「`ReadmeCueSink:`」7 か所。件 6＝`crates/areka-kanade/src/actor.rs` 750・760・769 行。件 9＝`menu/captions.rs` 87 行の `UNQUERIED_POPUPMENU_RESOURCES`。件 12 の記録 `menu_dismissed` は `trigger.rs` 492 行で今も DEBUG。件 13 ⑴＝問う名前の表 `crates/areka-kanade/src/schedule/resources.rs`（370 行）は `updatebutton.caption` だけ（49 行目）。件 3・8・11・12 は手つかず（件 12 は 10-22 のぱすたの実機で再現していない）。
- **触るファイル**: `menu/{trigger,win32,captions}.rs`（573・137・324）・`menu/trigger_show_tests.rs`（504）・`emo2_boot/readme_cue.rs`（140）・`crates/areka-kanade/src/actor.rs`（845・文言だけで増えない）・揃えるなら `actor_resources.rs`（107）・新しいテスト 1〜2 本。件 13 を入れるなら `schedule/resources.rs` とその兄弟のテスト・台帳 `shiori.toml` の 9 行。`spine.rs` には触らない。
- **規模**: 件 3〜6・8・9 だけなら 4〜6。件 13 ⑴（古い名前も問う）で +1。件 11・12 の再現と直しで +2〜3。件 13 ⑵（項目を足す）は大きければ切り出す（件 13 の節のとおり）。
- **先に要るもの**: 働きの上では無し。同じウェーブに置かない相手: `popup-menu-shell-look`（`menu/{mod,win32,plan}.rs`・件 11・12 をどちらが直すかは向こうの要件で決める・件 13 ⑵ のおすすめは向こうの件 15 と分け方を決める）・kanade の `actor.rs` の SHIORI の往復に手を入れる spec・`extra-character-windows`（`captions.rs`）・`tasktray-icon`（`menu/mod.rs`）・`menu-thumbnail-preview`（`menu/`）。v0.1.0 の `script-small-tags`・`install-before-boot`・`placement-measure-bake-once` とは重なり 0。1 本だけの席は取らない。
- **版の席**: 隙間に入れるなら件 3〜6・8・9 だけの形（4〜6・人の待ち無し・静かな机も前後の全体テストの比べも要らない）。件 11・12 は実機でメニューを開いて撮る仕事＝人の待ち（入力を送る許し）になりうるので後へ回す。件 13 も直した後に「．さくら」で SSP と見比べる確かめが要るので後へ回す。
- **優先度の区分**: C（記録の文言とテストの穴・件 13 は ukadoc の拾い残し）。件 11・12 だけ B。
- **要件定義のモデル**: Opus。
- **分割の案**: 新しくは切らない。隙間に入れるときは件 11〜13 を後へ回す。
- **見つけた穴・古くなった記述**: 本文の Approach・Scope・Adjacent の「`ghost-shell-balloon-switch`・`baseware-root-layout` がこれから行う」は古い（どちらも完了）。3 人目以降のキャラクターの窓（件 9 の `char{n≧2}`）を持つ spec は今も決まっていない。冒頭の残件の数の書き方（「3〜10 の 8 件」）は上の「残り」に読み替える。
