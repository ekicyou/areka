# Brief: areka-P0-tasktray-ghost-icon

> 2026-10-10 `/kiro-discovery`（開発者「タスクトレイの土台仕様を起票したうえで、上流の ukadoc 項目を後続仕様にせよ」）で起票した。土台は `areka-P0-tasktray-icon`。正典は ukadoc MCP で確かめた。着手時に引き直す。

## Problem

通知領域のアイコンと、マウスを重ねたときの文言を、ゴーストが決められない。ゴーストが descript の `icon,ファイル名` で自分のアイコンを指していても、台本が `\![set,tasktrayicon,…]` で状態に合わせて差し替えようとしても、areka では標準アイコンと既定の文言のままになる。タグは黙って消える。

## Current State

- 正典（要旨）:
  - descript の `icon,ファイル名`: そのゴーストのタスクトレイアイコン。値が無ければベースウェアの標準アイコン（ukadoc「ゴースト descript.txt」）。
  - `\![set,tasktrayicon,ファイル名.ico,テキスト(,--duration=待機時間(,--runcount=繰り返し回数))]`（ukadoc「さくらスクリプト」）:
    - 指したアイコンのファイルを通知領域のアイコンに当てる。ファイル名は、実行したゴーストのフォルダか、その `ghost/master` からの相対パス。
    - テキストはマウスを重ねたときの文言で、省ける。既定は「SSP/[起動中のゴースト名]」。
    - `--duration`（ミリ秒）を付けると、`ファイル名00.ico`・`ファイル名01.ico`…を数字の順に読んで、その間隔でこまを差し替える（SSP 2.5.58 から）。`--runcount` は繰り返しの回数で、無ければ繰り返し続ける（SSP 2.5.59 から）。
  - ukadoc「SHIORI/3.0」の `ValueNotify [SSP拡張 2.5.35]` は、通せるタグの一覧に `\![set,trayicon]` と書く（綴りが違う。別名か誤記かは未確認）。
  - プロパティ `ghostlist(ゴースト名/本体側名/パス).icon`: そのゴーストのアイコンのパス（ukadoc「プロパティシステム」）。
- 網羅台帳（`doc/ukadoc-coverage/ledger/`）: `icon,ファイル名`・`\![set,tasktrayicon,…]` は `absent`・持ち主なし。タグの行は「黙って壊れる・配送先の受け口が担当外の名前として記録を残す」と書いている。
- areka のコード: `.ico` を読む所も、アイコンのハンドルを作る所も無い。土台（`areka-P0-tasktray-icon`）が、アイコンの絵と文言を差し替える口を用意する。`\!` のタグは汎用の運び手 1 本で運び、受け口が名前で選ぶ作り。
- BTS の台帳（`areka-P0-ssp-bts-salvage` の `bts-ledger.md`）: アイコンの動く絵の 3 行（BTS 0000390・0000416・0000418）。0000418 は、台本で毎秒差し替えると `OnSurfaceRestore` が届かなくなるという指摘で、SSP は未対応のまま。

## Desired Outcome

- descript に `icon,ファイル名` を持つゴーストを起動すると、通知領域のアイコンがその絵になる。ゴーストを切り替えると、切り替え先の絵（無ければ標準アイコン）になる。
- 台本の `\![set,tasktrayicon,ファイル名.ico,テキスト]` で、アイコンと文言が替わる。テキストを省けば文言は既定のまま。
- 読めないファイル・フォルダの外を指すパスは、記録を残して何も替えない。
- 網羅台帳の 2 行が実物と合う。

## Approach

土台の「絵と文言を差し替える口」に、2 つの入り口をつなぐ。descript の `icon` は起動と切替のたびに読んで当てる。タグは `\!` の受け口を 1 つ足して当てる。`.ico` の読み込みは OS に任せる（自前で解かない）。

## 議題（要件の段で決める）

1. **動く絵の指定（`--duration`・`--runcount`）をどう扱うか**。開発者は 2026-10-10 に「今はタスクトレイは基本隠れるので、動く意味は無い」と述べた。OS は通知領域のアイコンを動画として受け取らず、動かすにはアプリがこまを差し替え続けるしかない。候補は「動かさず、最初のこま（`ファイル名00.ico`）だけ出す」「タグごと受けない」。網羅台帳には「対応しない」と理由を書く。
2. **タグで替えた絵と文言がいつまで効くか**。正典は書いていない。ゴーストの切替・終了で descript の絵と既定の文言に戻すのが自然。
3. **ファイル名の解き方**。ゴーストのフォルダと `ghost/master` のどちらを先に見るか、フォルダの外を指すパスをどう断るか（ほかのタグのファイルの解き方に合わせる）。
4. **`.ico` 以外の画像**（PNG など）を受けるか。正典は `.ico` と書く。
5. **`ValueNotify` の一覧の `\![set,trayicon]`** を別名として受けるか。
6. **文言の長さ**。OS の上限を超えた分の扱い。

## Scope

- **In**: descript の `icon,ファイル名` の読み取りと適用／`\![set,tasktrayicon,ファイル名.ico,テキスト]` の受け口／`.ico` の読み込み／切替・終了での戻し／議題 1 の結論の実装（動かさないなら、指定の読み捨てと記録）／網羅台帳の 2 行／決定論テストと実機の確かめ。
- **Out**: 土台そのもの（`areka-P0-tasktray-icon`）／`icon.minimize,ファイル名`（`areka-P0-tasktray-minimize`）／`\![set,trayballoon,…]`（`areka-P0-tasktray-balloon`）／プロパティ `ghostlist(…).icon`（プロパティの一覧の spec）／`ValueNotify` そのもの（BTS の台帳の候補）。

## Boundary Candidates

- descript の読み取り（`icon` のキー）。
- `\!` の受け口（`set,tasktrayicon`）と引数の判定。
- `.ico` の読み込みとパスの解き方。

## Out of Boundary

- 通知領域の部品（土台が持つ。足りない口が在れば土台の側へ戻して足す）。
- メニュー。

## Upstream / Downstream

- **Upstream**: `areka-P0-tasktray-icon`。
- **Downstream**: `areka-P0-tasktray-minimize`（`icon.minimize` が同じ読み込みを使う）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: 引数の判定を純粋な関数へ取り出す `check-script-arg-checks`（受け口を足すときの形を合わせる）。台本が外へ影響する度合いを段に分ける `script-impact-tiers`（アイコンと文言の差し替えをどの段に置くか）。

## Constraints

- 黙って壊れる経路を残さない（読めない・断った・読み捨てたを記録する）。
- ゴーストのフォルダの外のファイルを読ませない。
- 動く GIF に対応しない方針（roadmap「動く画像」）と同じく、対応しないものは網羅台帳に理由を書く。
