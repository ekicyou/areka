# Brief: shell-tooltip

> 2026-10-05 `/kiro-discovery` で起票（「バルーンのリンクと OS の連携」の議題 6・開発者裁定＝案 A＝起票して `wintf-tooltip` の後に作る）。「バルーンのリンクと OS の連携」の 7 本の 1 本（roadmap の同名の節が分け方の正本）。

## Problem

- **利用者**: キャラクターの当たり判定に説明を添えたいシェル作者・ゴースト作者と、触れる場所を知りたい利用者。
- 正典（ukadoc）には 3 つの道がある。
  - surfaces.txt の **tooltipブレス**（`sakura.tooltips`・`kero.tooltips`・`char*.tooltips`）。中の行は「当たり判定名,表示内容」＝当たり判定の上に来たときに出す文字。例: `Head,頭をなでる`。
  - SHIORI リソース `tooltip`＝シェルにおけるツールチップ内容の取得（Reference0・1＝マウスの座標・Reference3＝本体 0／相方 1／それ以降・Reference4＝当たり判定の識別子）。「シェル側設定がある場合はそちらが優先される」。
  - プロパティ `currentghost.seriko.tooltip.scope(ID).textlist(当たり判定名).text` ほか（[SET有効]・空文字の SET で定義を消す）。
- areka ではどれも未実装で、tooltipブレスは**黙って吸収**される（記録なし）。

## Current State

- tooltipブレス: `crates/areka-parsers/src/shell/` の `dispatch_block` が名前で振り分ける塊に入っておらず、塊ごと吸収される。網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml` の「当たり判定名,表示内容」の行は「無い」・引受先なし。
- SHIORI の `tooltip`: `crates/areka-sylphya/src/vocab/shiori_resource.rs` の語の表だけ。台帳 `shiori.toml` で「語の表だけ」・引受先なし。
- プロパティ: `crates/areka-sylphya/src/vocab/dotted.rs` に `seriko.tooltip.text`・`seriko.tooltip.name`（`RuntimeCommand`）の語がある。台帳 `property.toml` の `currentghost.seriko.tooltip.*` の 5 行の状態は要件の段で確かめる。
- 当たり判定はキャラクター窓の当たりの仕組み（`collision`・`surface-element-nesting` の `HitRegions`）。`collisionex` の形は `collisionex-regions` が作る。

## Desired Outcome

- キャラクターの当たり判定にマウスが止まったら、次の順で決めた文字をツールチップに出す。
  1. プロパティで差し替えた文字。
  2. tooltipブレスの文字。
  3. ゴーストへ `tooltip` を尋ねて返った文字。
  4. どれも無ければ出さない。
- 1 と 2 の優先の順は要件の段で ukadoc を引き直して確定する。
- tooltipブレスを読めない行は記録を出す（黙って吸収しない）。

## Approach

- `wintf-tooltip` の口を使う。静的な登録（tooltipブレス・プロパティ）と動的な表示（SHIORI の `tooltip` の答え待ち）の両方。
- tooltipブレスの読み込みは `areka-parsers` の shell の塊の振り分けに 1 つ足す。

## Scope

- **In**: tooltipブレスの読み込み・SHIORI の `tooltip`・プロパティの SET と GET・キャラクター窓の配線・決定論テスト・台帳の行。
- **Out**:
  - バルーンのツールチップ（`balloon-link-hover`）。
  - マウスカーソルの差し替え（`mousehover*` などのカーソルの項目）。

## Boundary Candidates

- tooltipブレスの読み込み（parsers・純粋）。
- 文字の決め方と配線（キャラクター窓・SHIORI・プロパティ）。

## Out of Boundary

- 当たり判定の形（`collisionex-regions`）。

## Upstream / Downstream

- **Upstream**: `wintf-tooltip`。`collisionex-regions` が先に入れば `collisionex` の形にも出せる。
- **Downstream**: なし。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `collisionex-regions`（`shell/decode.rs` を触る）・`currentghost-property-tree`（プロパティの動く値の口）。

## Constraints

- シェルの element の列（`shell/{model,decode}.rs`）とプロパティの動く値の列に掛かる見込み。着手の前に照合する。
- 段: その他（バルーンのリンクとは別の窓・急ぐ理由が無い）。規模の見込み M（8〜12）。


## 2026-10-10 棚卸㉓の申し送り

- **実機の確かめに 1 項目足す: 主画面より上に置いた画面でツールチップを出す**。完了 `wintf-tooltip` は、ツールチップの位置を OS へ渡すときに 2 つの座標を 16 ビットずつに詰める（`crates/wintf/src/ecs/tooltip/os.rs` の `make_lparam`・負の座標は 2 の補数のまま）。OS が符号付きで読むかは実機でしか決まらない。同 spec の `tasks.md` の Implementation Notes は、3.1 で「負の座標の画面で出す確かめを入れる」と書き、6.1 で「y が負の画面が無いので負の y の詰め方は未確認」と残した。
- areka からこの口を最初に使う spec（本 spec か `balloon-link-hover`）の実機の確かめで、主画面の上に画面を置き、そこでツールチップの位置が合うかを見る。ずれたら wintf の側の不具合として起票する（本 spec の中では直さない）。

## 2026-11-01 棚卸㉝の再測定（main 13b80ab1）
- **前提の変化**: 追跡を作る 2 本が着地した（✅ `mouse-click-wheel-events` 10-24・✅ `shell-mouse-cursor` 10-30）。窓ごとの「今どの当たり判定の上に居るか」は `crates/areka/src/input_events/hover_region.rs`（105・`HoverState`・置き場は `MouseWiring.hover`）に在り、説明に「後の `shell-tooltip` は `HoverState` を読む口を足して使う」と書いてある＝この spec は読むだけ。wintf のツールチップの口を areka で使うコードは今も 0。tooltipブレスは今も記録なしに捨てられる。✅ `popup-menu-extra-resources`（11-01）が SHIORI のリソースの許可の表（`crates/areka-kanade/src/schedule/resources.rs` の `ALLOWED_RESOURCE_IDS`・名前を固定したテストつき）を 15 名にした＝`tooltip` を足すと 16 名。照会の組み立て `resource_get`（110 行目）と kanade の `KanadeMsg::ResourceQuery`（`msg.rs` 252 行目・欄は `ids` と返信端だけ）は今も Reference を付けない形だけ＝付ける形を 1 つ足す。
- **触るファイル**（行数・プロパティを外した形）: `crates/areka-parsers/src/shell/` の新しい転記のファイルと `shell/mod.rs`（80）／`crates/areka-emo-present/src/shell_target.rs`（**951＝上限の近く**・✅ `shell-scale-linkage` が 757 から増やした＝足す前に中のテストか処理を兄弟へ出す）／`crates/areka/src/input_events/` の新しいファイルと `input_events/mod.rs`（612）・`hover_region.rs`（105）・`crates/areka/src/emo2_boot/hit_region.rs`（167）／`crates/areka-kanade/src/schedule/resources.rs`（407）・`actor_resources.rs`（107）・`msg.rs`（586）／台帳 `assets.toml`・`shiori.toml` の行。`emo2_boot/mod.rs`（828）は配線を `input_events/` に置けば触らない。依存の追加・emo-text の新しいファイルは無い。
- **規模**: 8〜12 タスク（`shell_target.rs` を先に分ける 1 つを含め、追跡を使う分で下の方）。プロパティまで持つと 13〜16。
- **先に要るもの**: 働きの上では無し（入力の列の最後で、先頭）。プロパティの部分だけ `currentghost-property-tree`（未着手）＝外す。ファイルの上では `balloon-link-hover`（同じ wintf の口・`resources.rs`）・`shell-hover-animation`（`input_events/` の追跡）・`collisionex-regions`（当たり判定）・kanade の `msg.rs` を触る spec と同じウェーブに置かない。
- **`balloon-communicatebox` との重なり**: 配線を `input_events/` に置けば、あちらの一覧（バルーンの読み手・`consumer_ledger*.rs`・`emo2_boot/{mod,runtime_boot}.rs`・`schedule/events.rs`・`status.rs`・`menu/mod.rs`）と共有のファイルは無い。ただし、あちらが入力欄の文字を「マウスとキーの知らせの今の道」で返す形にすると `input_events/mod.rs` で重なり得る＝並べるなら設計の前に照らす。
- **人の手・机**: **人の手の実機の確かめが要る**（棚卸㉓の申し送りの「主画面より上に置いた画面での位置」は画面の並べ替えが要る）＝席の空きに挟む候補にならない（棚卸㉖のまま）。マウスを当たり判定に止めて出るかの確かめは排他テストの机で SendInput。静かな机・前後の全体テストの比べは要らない。
- **優先度の区分**: A のまま（開発者 10-05「キャラクター窓のツールチップも起票する」）。段は「その他」・版に縛らない（ぱすたはツールチップを使わない）。
- **要件定義のモデル**: Opus のまま（wintf の口・当たり判定の追跡・リソースの照会のどれも先例がある）。
- **分割の案**: 無し（プロパティは `currentghost-property-tree` 側へ）。
- **見つけた穴・古くなった記述**: 正典は「シェル側設定がある場合はそちらが優先される」＝tooltipブレスが SHIORI の `tooltip` より先で、プロパティの差し替えとの順だけが議題に残る（棚卸㉓のまま）。台帳 `property.toml` の `currentghost.seriko.tooltip.*` の 5 行（1007〜1063 行目）の持ち主は `currentghost-property-tree`＝本 spec は文字の表と差し替えの口まで。本文 Constraints の「シェルの element の列（`shell/{model,decode}.rs`）に掛かる見込み」は 2 つ目の転記にすれば掛からない。本文 Out の「マウスカーソルの差し替え」の持ち主は ✅ `shell-mouse-cursor`（着地済み）。

## 2026-11-01 棚卸㉝の後の開発者の決め

2026-11-01 開発者の承認（棚卸㉝の後・ツールチップは「境界領域です。リアクター描画にした方が良い可能性がある。WinUIで描画できる要素ですからね」）＝版に縛らない列から windows-reactor 待ちの段へ移した。中身の大半はツールチップを出すことで、読み取り（surfaces.txt の tooltipブレス・SHIORI の `tooltip`）は小さいので、描き方が決まってから一緒に作る。
