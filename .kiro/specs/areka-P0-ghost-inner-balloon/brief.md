# Brief: areka-P0-ghost-inner-balloon

> 2026-10-05 `/kiro-discovery`（再入）で起票。`ghost-standard-balloon` の要件の討議（議題 2＝`default.balloon.path` の起点）で、開発者が「`<根>/ghost/claudia/balloon/claudia_vertical` も在ってよい。shell フォルダがあるんだから balloon フォルダがあっても良い」「nar 仕様にも関わる。nar 修正と一緒に別 spec に」と決めた。**ゴーストが自分のフォルダの中にバルーンを抱えられるようにする**＝ukadoc に無い areka 独自の拡張。ソースの指し先は 2026-10-05（main `ec072853`）の実測＝着手時に引き直すこと。

## Problem

- **ゴーストの作者**: そのゴースト専用のバルーンを、ゴーストと一体で配れない。
  - 同梱のバルーンは根の置き場（`<根>/balloon/`）へ入り、全ゴーストの共有物になる。同じフォルダ名のバルーンを別のゴーストが入れると、上書きされるか、別の版に替わる。
  - シェルは `<ゴースト>/shell/` に抱えられるのに、バルーンには同じ置き場が無い。
- **利用者**: ゴーストを消しても、そのゴースト専用のバルーンが根の置き場に残る。

## Current State

- **正典（ukadoc）**: ゴーストの中にバルーンを置く決まりは書かれていない（2026-10-05 に ukadoc MCP で `default.balloon.path` を引き直した）。
  - `default.balloon.path,パス`: 「標準で使用するバルーンの相対パス。」起点は書いていない。
  - 同梱（`balloon.directory`）の入り先は、バルーンの置き場である。
- **areka（2026-10-05）**:
  - **一覧**: `crates/areka-ghost/src/catalog.rs` の `list_balloons` は `<根>/balloon/` の直下だけを数える。メニューと `\![change,balloon,バルーン名]` はこの一覧に依る。
  - **記憶**: Ghost の範囲の `areka.last.balloon` は「前回のバルーンのフォルダ名」1 つ（`crates/areka-sylphya/src/persist/`）。置き場を区別する欄は無い。
  - **起動時の解決**: `crates/areka/src/boot_resolve.rs` の `resolve_balloon` は、フォルダ名の一覧と突き合わせる。根の置き場の外のバルーンを使えるのは引数だけ。
  - **インストール**: 同梱のバルーンは `<根>/balloon/<名>` へ入る（`crates/areka-nar/`）。ゴーストの書庫の中に `balloon/` というフォルダが在ったときに、それがゴーストのフォルダの中へそのまま入るのかは未確認（着手時に確かめる）。
- **`ghost-standard-balloon` の決め**（2026-10-05 の要件の討議）: `default.balloon.path` の値はフォルダ名 1 段で、根の置き場とだけ突き合わせる。区切り・`..`・絶対パスを含む値は当たらないとして記録する。ゴーストの中の `balloon/` は見ないと `doc/COMPAT_ARCHITECTURE.md` §8 に書く。

## Desired Outcome

- ゴーストのフォルダの中の `balloon/<名>/`（例: `<根>/ghost/claudia/balloon/claudia_vertical`）に置いたバルーンを、そのゴーストが使える。
- そのバルーンが、起動時の解決・メニュー・`\![change,balloon,バルーン名]`・前回の記憶のどれでも、根の置き場のバルーンと同じように扱える（選べる・覚えられる・戻れる）。
- 書庫（nar）から、ゴーストの中の `balloon/` へバルーンを入れる道が在る。
- ゴーストの中と根の置き場に同じ名前が在るときの優先が決まっていて、`doc/COMPAT_ARCHITECTURE.md` §8 に記されている。
- 他のゴーストからは、そのバルーンが見えない（メニューに出ない）。

## Approach

- バルーンの一覧を「根の置き場＋今のゴーストの中の `balloon/`」の 2 か所から作り、どちらの置き場のものかを素性に持たせる。
- 起動時の解決の各段（記憶・descript の `balloon`／`default.balloon.path`・同梱）は、その一覧と突き合わせる。`ghost-standard-balloon` の値の書き方（フォルダ名 1 段）は変えない。
- **要件の段の議題**（ukadoc に無いので areka が決める）:
  1. 同じ名前が両方に在るとき、どちらを先にするか。起票時の見立ては「ゴーストの中が先」（作者が版まで決めて入れたもの・シェルと同じ「そのゴーストの持ち物」）。
  2. 記憶（`areka.last.balloon`）で 2 つの置き場をどう区別するか（欄を足すか、値の書き方で分けるか、区別せず 1 の優先で引き直すか）。
  3. 書庫からの入れ方。ゴーストの書庫の中の `balloon/` をそのまま入れるだけで足りるか、`install.txt` に areka 独自の鍵を足すか。ukadoc の鍵の意味は変えない。
  4. ネットワーク更新・上書きインストール・ゴーストの削除での扱い（ゴーストと一緒に替わる・消えるでよいか）。
  5. `default.balloon.path` で区切りを含む値（`balloon/claudia_vertical` など）を、ゴーストのフォルダからの相対パスとして読むか。読むなら `ghost-standard-balloon` の「当たらないとして記録」を上書きする。
  6. シェルに紐づくバルーン（`shell-companion-balloon`）が、ゴーストの中のバルーンを指せるか。

## Scope

- **In**:
  - ゴーストの中の `balloon/` の列挙と、素性での置き場の区別。
  - 起動時の解決・ゴーストの切替・メニュー・`\![change,balloon,バルーン名]`・記憶での扱い。
  - 書庫からゴーストの中の `balloon/` へ入れる道（`areka-nar`）。
  - 決定論のテスト。
  - `doc/COMPAT_ARCHITECTURE.md` §8 と `dist/README.txt` などの作者向けの説明（areka 独自の拡張であること）。
- **Out**:
  - 根の置き場のバルーンの扱い（変更 0）。
  - ukadoc の `install.txt` の鍵の意味の変更。
  - `recommended.balloon`／`recommended.balloon.path`（roadmap の覚え書き）。
  - シェルの中にバルーンを抱えること（`<ゴースト>/shell/<名>/balloon/`）。要望が出てから。
  - シェルの絵の中に台詞を書くこと（完了 `shell-balloon`）。名前が似ているが別物。

## Boundary Candidates

- ゴーストのフォルダの読み手（`areka-ghost` の `catalog.rs`）＝列挙と素性
- 起動時のバルーンの解決（`areka` の `boot_resolve.rs`・`boot_config.rs`）
- 実行中のバルーンの切替（`emo2_boot/shell_balloon_resolve.rs`・メニュー）
- 記憶（`areka-sylphya` の `persist`）
- インストール（`areka-nar`）

## Out of Boundary

- `ghost-standard-balloon` が決める段の並びと、descript の 2 鍵の値の書き方（議題 5 を除く）。
- `shell-companion-balloon` が決める、シェルに紐づくバルーンの持ち方。
- バルーンの見た目の鍵（`char*.balloon.*` など）。

## Upstream / Downstream

- **Upstream**: `ghost-standard-balloon`（段の並び・descript の 2 鍵）・`shell-companion-balloon`（同じ `catalog.rs`・`boot_resolve.rs`・`boot_config.rs` を触る＝直列で後ろ）・完了 `install-companion-reading`・完了 `ghost-shell-balloon-switch`。
- **Downstream**: なし（起票時点）。

## Existing Spec Touchpoints

- **Extends**: なし。`ghost-standard-balloon` が「ゴーストの中の `balloon/` は見ない・別 spec」と書いた先を引き取る。
- **Adjacent**: `shell-companion-balloon`（同じ解決の鎖）・`install-live-target-hazards`（使用中のバルーンへの上書き）。

## Constraints

- ukadoc に無い areka 独自の拡張である。ukadoc の鍵の意味を変えず、SSP 向けに書かれたゴーストの動きを変えない。
- 「起動はできるが、メニューに無く、記憶もできず、一度切り替えたら戻れないバルーン」を作らない（一覧・記憶・切替を同じ spec で揃える）。
- 利用者が選んだもの（記憶・引数）を作者の指定で上書きしない。
- 決定論のテスト網羅は必達。ログの無い失敗の経路を作らない。検体と一時フォルダはワークツリーの `target\` の下だけ。
- 段は**優先**（バルーン関係）・並びは `shell-companion-balloon` の後（棚卸で決める）・規模 M〜L（12〜18 タスク・議題 2 と 3 の答えで動く）・Fable 推奨。


## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- 前提の変化: 待っていた `ghost-standard-balloon` が着地した（10-07）。起票のときの「決め」はそのまま実装された。
  - descript の `balloon`（`name` → フォルダ名の順に突き合わせ）と `default.balloon.path`（フォルダ名 1 段だけ。区切りや `..` を含む値は当たらないとして記録）が鎖の段になった。「ゴーストの中の `balloon/` は読まない」は `crates/areka/src/boot_config.rs` の `resolve_balloon_for_ghost` の説明と `doc/COMPAT_ARCHITECTURE.md` §8 に書いてある。
  - 一覧の項目 `BalloonEntry` は場所と素性（フォルダ名・`name`）を持つ。ただし鎖（`crates/areka/src/boot_resolve.rs` の `resolve_balloon`）は、決めた場所を「根＋フォルダ名」から組み直している＝置き場を 2 つにするなら、項目の場所をそのまま使う形へ直す。
  - **未確認だった点が分かった**: 書庫の中の `balloon/` は、`install.txt` が同梱の取り出し元（`balloon.source.directory`）として名指ししなければ、ゴーストのフォルダの中へそのまま入る（`crates/areka-nar/src/plan.rs` は名指しされたフォルダだけを本体から除く）＝議題 3 は「そのまま入れるだけ」で足りる見込み。
  - 一覧を引く呼び手が 1 つ増えた: `\![open,explorer,…]` の名前引き（`crates/areka/src/readme/opener.rs` の `named_folder`）。
- 触るファイル: `crates/areka-ghost/src/catalog.rs`（414）・`crates/areka/src/boot_resolve.rs`（609）・`boot_config.rs`（503）・`emo2_boot/shell_balloon_resolve.rs`（217）・`menu/balloon_frame.rs`・`update/desk.rs`（`updateother` のバルーンの引き方）・`install/procedure.rs`（入れた後の一覧）・`readme/opener.rs`・（記憶の形を変えるなら）`crates/areka-sylphya/src/persist/`・（書庫に鍵を足すなら）`crates/areka-nar/src/{manifest,plan}.rs`・`doc/COMPAT_ARCHITECTURE.md` §8・`dist/README.txt`。
- 規模: 12〜18 タスク（議題 2 と 3 の答えで動く。3 が「そのまま入れる」なら下の方）。
- 先に要るもの: 働きの上では無し（`shell-companion-balloon` を待つ理由は、同じファイルを触ることだけ。議題 6 は後から着地する側が答える）。ファイルの重なりは次のとおり。
  - `shell-companion-balloon`・`baseware-root-list`（`catalog.rs`・`boot_resolve.rs`・`boot_config.rs`＝この 3 本は直列）。
  - `baseware-root-list`（さらに `shell_balloon_resolve.rs`・`menu/balloon_frame.rs`・`update/desk.rs`・`install/procedure.rs`・`readme/opener.rs`）。
  - `network-update-canon-order`・`update-check-options`（`update/desk.rs`）。
  - `emily-ghost-verification`（`areka-nar` の `plan.rs`・書庫に鍵を足す場合だけ）。
- 優先度の区分: A（開発者「shell フォルダがあるんだから balloon フォルダがあっても良い」「別 spec に」）。
- 要件定義のモデル: Fable（ukadoc に無い独自の拡張・議題 6 つ）。
- 分割の案: なし。
- 見つけた穴・古くなった記述:
  - 本文「Current State」の「`resolve_balloon` は、フォルダ名の一覧と突き合わせる」は古い（今は `BalloonEntry` の列）。
  - `baseware-root-list` も「項目がどの置き場のものか」を持たせる。先に着地する方が素性に置き場の欄を作り、後の方は種類を足すだけにする（2 本で別々の形を作らない）。
