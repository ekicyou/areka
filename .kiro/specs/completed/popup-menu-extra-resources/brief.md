# Brief: popup-menu-extra-resources

2026-10-31 `popup-menu-residue` の完了時の棚卸で起票（同 spec の件 13・設計の要件 9.2 の決めにより新しい spec 1 本）。出どころは `shiori2-protocol` のタスク 5.4（2026-10-24）。原文は `.kiro/specs/completed/popup-menu-residue/brief.md` の「2026-10-24 追記: 件 13」の節。

## Problem
「．さくら」（華和梨・SHIORI/2.x）のような古いゴーストは、右クリックメニューの項目名や項目の有無を、areka が問わない名前の資源で持っている。areka で開くと、そのゴーストらしい項目名にならず、SSP では出る項目そのものが無い。

## Current State
- areka が問う資源の名前は `crates/areka-kanade/src/schedule/resources.rs` の `ALLOWED_RESOURCE_IDS` にある。
- ⑴ 古い名前: 「ネットワーク更新」の項目は `updatebutton.caption` だけを問う。「．さくら」の `updatebuttoncaption` は使われず、areka の素の言葉のままになる。ukadoc は `updatebuttoncaption` を別の項目として載せている（<https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html#updatebuttoncaption>）。
- ⑵ 項目が無い: 消滅通告（`vanishbuttoncaption`・`vanishbuttonvisible`）・おすすめ（`sakura.recommendbuttoncaption`・`kero.recommendbuttoncaption`・`sakura.recommendsites`・`kero.recommendsites`）・ポータル（`sakura.portalbuttoncaption`・`sakura.portalsites`）の項目を areka のメニューは持たない。
- 台帳 `doc/ukadoc-coverage/ledger/shiori.toml` では、この 9 つの名前はどれも `vocabulary-only`・持ち主なし。
- 実機（`shiori2-protocol` 5.1・2026-10-24）では右クリックメニューを開いていないので、画面ではまだ見ていない。

## Desired Outcome
- ⑴ 古い名前の資源を持つゴーストで、「ネットワーク更新」の項目名がゴーストの言葉になる。
- ⑵ 消滅通告・おすすめ・ポータルの項目が、ゴーストが資源を持つときに SSP と同じ形でメニューに出る（出す・出さないの判断も ukadoc どおり）。
- 9 つの名前の台帳の行が、持ち主と状態の付いた行になる。
- 「．さくら」で右クリックメニューを開き、項目名と項目の有無を SSP と見比べて同じになる。

## Approach
⑴ を先に入れ（問う名前を足して、新しい名前が無いときに古い名前で引く）、⑵ は要件の段で大きさを測る。項目を足す仕事が大きすぎると分かったら、⑵ を別の spec に切り出す（`popup-menu-residue` の件 13 の節のとおり）。意味は ukadoc から取り、ukadoc で読めない点（古い名前と新しい名前が両方あるときの優先など）は議題にする前に SSP の MCP で測る。

## Scope
- **In**: ⑴ `updatebuttoncaption` を問う直し、⑵ 消滅通告・おすすめ・ポータルの項目、問う名前の表と兄弟のテスト、台帳の 9 行。
- **Out**: メニューの見た目（オーナードロー・着せ替えのメニュー＝`popup-menu-shell-look`）、3 人目以降のキャラクター窓のメニュー（`extra-character-windows`）。

## Boundary Candidates
- 問う名前の表（kanade の `schedule/resources.rs`）と古い名前の引き方
- メニューの組み立て（areka の `menu/`・項目を足す）
- おすすめ・ポータルのサイトの一覧の読み方と、選んだときに開く道

## Out of Boundary
- 右クリックメニューの色・絵・文字の描き方（`popup-menu-shell-look`）
- 省略した形のメニューの作り分け（`popup-menu-residue` が作り分けないと裁定済み）

## Upstream / Downstream
- **Upstream**: 完了した `popup-menu-residue`（問わない名前の表 `UNQUERIED_POPUPMENU_RESOURCES`・台帳の決め）、完了した `shiori2-protocol`（「．さくら」の観察）。
- **Downstream**: なし（見込み）。

## Existing Spec Touchpoints
- **Extends**: なし（新しい spec）。
- **Adjacent**: `popup-menu-shell-look`（`menu/{mod,win32,plan}.rs` を分け合う。同じウェーブに置かない。同 spec の件 15 は SSP がメニューを組むときに `sakura.recommendsites` を問うことを見ているので、おすすめの項目の分け方を要件の段で決める）、`extra-character-windows`（`menu/captions.rs`）、`menu-thumbnail-preview`（`menu/`）、`tasktray-icon`（`menu/mod.rs`）。

## Constraints
- **優先度の区分**: C（ukadoc の拾い残し）。
- **規模**: S〜M（⑴ だけなら 2〜3・⑵ を入れると 8〜12 の見込み・超えたら ⑵ を切り出す）。
- **確かめ**: 直したら「．さくら」で右クリックメニューを開き、項目名を SSP と見比べる（画面を操る確かめ＝排他テストの机を取る）。
- **要件定義のモデル**: Opus（先例のある資源の名前とメニューの項目の足し方で決まる）。

## 2026-10-31 棚卸㉜の再測定（main d374f901）
- **前提の変化**: 起票の後で触るファイルは動いていない。`crates/areka-kanade/src/schedule/resources.rs` 370 行（`updatebutton.caption` は 49 行）・`crates/areka/src/menu/captions.rs` 324（問わない名前の表 `UNQUERIED_POPUPMENU_RESOURCES` は 87 行）・`menu/mod.rs` 386・`menu/plan.rs` 139・`menu/win32.rs` 151・`menu/update_frame.rs` 37。台帳 `shiori.toml` の 9 行（`updatebuttoncaption` は 15329 行ほか）は今も `vocabulary-only`・持ち主なし。
- **触るファイル**: ⑴＝`resources.rs`・`menu/captions.rs`（古い名前で引く所）と兄弟のテスト・台帳の 1 行。⑵＝加えて `menu/{plan,mod,win32}.rs` と項目の枠のファイル（新しい `*_frame.rs`）・サイトの一覧を開く道・台帳の 8 行。
- **規模**: ⑴ 2〜3・⑴＋⑵ 8〜12 のまま。
- **先に要るもの**: 働きの上では無い（先頭）。v0.1.0 の `shell-scale-linkage` とは重なり 0（向こうの kanade は `schedule/events.rs` で、`menu/` に触らない）。`popup-menu-shell-look`（裁定待ち・ウェーブの外）・`extra-character-windows`・`menu-thumbnail-preview`・`tasktray-icon` と同じウェーブに置かない。`dev-folder-alias` は `menu/roots_frame.rs`・`menu/shell_frame.rs` を挙げる＝⑵ で `menu/` に枠を足すときは照らす。
- **隙間（割り込み）の席**: 全体（上限 12）は入らない。⑴ だけなら入る（2〜3・「．さくら」で右クリックメニューを 1 回開いて SSP と見比べる短い確かめ 1 回）。⑵ は要件の前に SSP の MCP の測り（古い名前と新しい名前が両方あるときの優先・`*.recommendsites` を問う時）が要る。
- **優先度の区分**: C のまま（ukadoc の拾い残し）。
- **要件定義のモデル**: Opus のまま。
- **分割の案**: ⑴ と ⑵ を分けるかは brief のとおり要件の段で決める。
- **見つけた穴**: 台帳には 9 つのほかに `char*.recommendbuttoncaption`・`char*.recommendsites`・`char*.recommendsites.caption`（`shiori.toml` 12525〜12573 行・同じく持ち主なし）がある＝3 人目以降の窓の名前で、`extra-character-windows` に回すか ⑵ に入れるかを決める。

## 2026-10-31 棚卸㉜の後の開発者の決め
- 開発者「メニュー関係はreactor解放されたら大改造するので、内容によっては後回しになりますが」。推奨の案 1 を承認した。
- **この spec で取るのは ⑴ だけ**（古い名前 `updatebuttoncaption` を問う・kanade の `schedule/resources.rs` と `menu/captions.rs` の引き方・台帳の 1 行）。⑴ は SHIORI に何を問うかが大半で、メニューを作り直しても残る。
- **⑵（消滅通告・おすすめ・ポータルの項目）は windows-reactor の作り直しの後へ回す**。`menu/{plan,mod,win32}.rs` に項目を組む仕事は作り直しで捨てるため。完了の棚卸で ⑵ を roadmap の「windows-reactor 待ち」の段へ渡す（新しい brief にするか、この brief の残りとして置くかは完了の棚卸で決める）。`char*.recommend*` の 3 つの名前も ⑵ と一緒に回す。

## 2026-11-01 完了の棚卸: ⑵ を windows-reactor 待ちの段へ渡す
- **渡した先**: `.kiro/steering/roadmap.md` の「windows-reactor 待ち」の段（2026-11-01）。新しい brief は立てず、この節を ⑵ の残りの正本とする（メニューは windows-reactor の後に作り直すので、今 spec に切り出しても組み方の前提が変わる）。
- **渡す理由**: 開発者の決め（2026-10-31「メニュー関係はreactor解放されたら大改造する」）。`menu/{plan,mod,win32}.rs` に項目を組む仕事は作り直しで捨てるため。
- **渡すもの**:
  - ⑵ の 8 つの名前: `vanishbuttoncaption`・`vanishbuttonvisible`・`sakura.recommendbuttoncaption`・`kero.recommendbuttoncaption`・`sakura.recommendsites`・`kero.recommendsites`・`sakura.portalbuttoncaption`・`sakura.portalsites`。
  - 3 人目以降の窓の名前 3 つ: `char*.recommendbuttoncaption`・`char*.recommendsites`・`char*.recommendsites.caption`。
  - `readmebuttoncaption`（要件討議の決めの 2 で本 spec では取らなかった。SSP は「．さくら」に `readmebuttoncaption` と `readmebutton.caption` の両方を問う＝検証の記録 4.2）。
  - おすすめ・ポータルのサイトの一覧の読み方と、選んだときに開く道（未決）。
  - `popup-menu-shell-look` の件 15（SSP がメニューを組むときに `sakura.recommendsites` を問う）との分け方（未決）。
- **測った事実**（検証の記録 4.2）: SSP は「．さくら」に `updatebutton.visible`→`updatebuttoncaption`→`readmebutton.visible`→`readmebuttoncaption`→`readmebutton.caption`→`vanishbuttonvisible`→`vanishbuttoncaption` の順に問い、「燃やすゾ！」「さくらのお勧め」「ポータルサイト」の項目を出す。
