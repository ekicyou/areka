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


## 2026-10-11 棚卸㉔の再測定（main `7c5469f3`）
- 前提の変化: 触るファイルは棚卸㉓から変わっていない（`catalog.rs` 414・`boot_resolve.rs` 609・`boot_config.rs` 503・`update/desk.rs` 703・`readme/opener.rs` 401）。同じ鎖に 3 本が加わった（`user-data-root`・`write-failure-notice`・`default-balloon-selfmade`＝10-10 起票）。
- 触るファイル・規模: 棚卸㉓のまま（12〜18 タスク）。
- 先に要るもの: 働きの上では無し。ファイルの上では、カタログの鎖の席を待つ。
- カタログの鎖の順の案: `baseware-root-list` → `user-data-root` → `ghost-inner-balloon` → `write-failure-notice` → `shell-companion-balloon`＝この spec は 3 番目。
  - 一覧の項目の「どの置き場のものか」の欄は `baseware-root-list` が作り、この spec は種類を 1 つ足すだけになる。
  - `user-data-root` を先にするのは、winget の案内と次の版がそれを待つため。
- ファイルの重なり（足す分）:
  - `user-data-root`（`catalog.rs`・`boot_resolve.rs`・`boot_config.rs`）。
  - `write-failure-notice`（`boot_resolve.rs`。向こうが触るのは起動中の印と記録の関数だけで、バルーンを決める鎖には触らない）。
  - `default-balloon-selfmade`（`boot_resolve.rs` の既定の名前の定数と、そのテスト）。
  - `emily-ghost-verification` とは、書庫に鍵を足さない限り 0（棚卸㉓のまま）。
- 優先度の区分: A。要件定義のモデル: Fable（どちらも変わらず）。
- 見つけた穴・古くなった記述: 本文「Constraints」の「並びは `shell-companion-balloon` の後」は古い（待つ理由はファイルだけと棚卸㉓で分かった。区分 A のこちらが先）。

## 2026-10-18 棚卸㉗の再測定（main 2d86dbff）
- 前提の変化: カタログの鎖の 3 本（`baseware-root-list`・`user-data-root`・`write-failure-notice`）が着地した。一覧は `crates/areka-ghost/src/catalog_roots.rs`（252）の `list_balloons` が根の並びの `<根>/balloon/` を集め（`catalog.rs` 453 の `scan_balloons`）、起動の決め方は `crates/areka/src/boot_resolve.rs`（676）の `resolve_balloon` と `boot_config.rs`（467）の `resolve_balloon_for_ghost`。ゴーストの中は今も見ない。
- 棚卸㉗で足した範囲（理由と証拠）: **ゴーストのフォルダの中の、`balloon/` の外に置いたバルーンを名前で見つける**へ広げる。ぱすたは縦書きのバルーンを `<ゴースト>/pasta_tategaki/`（ゴーストのフォルダの直下）に置き、`install.txt` に `balloon.directory` を書かず、ゴーストの descript で `balloon,窓際のぱすたさん縦書き`（バルーンの `name`）と名前で指す（ゴースト「窓際のぱすたさん」の `install.txt`・`ghost/master/descript.txt`・`pasta_tategaki/descript.txt`）。areka は `<根>/balloon/` しか見ないので、既定の横書きのバルーンへ落ちる＝ぱすたで見込みいちばん大きく見える崩れ。本文の前提「`<ゴースト>/balloon/<名>`」だけでは当たらない。
  - 書庫の入れ方は今のままで足りる（`balloon.directory` で名指しされないフォルダは、ゴーストのフォルダへそのまま入る＝棚卸㉓）。
  - 正典: ukadoc「全体の構成」は、バルーンはゴーストごとでなく全ゴーストで共有されると書き、ゴーストの中のバルーンを書いていない＝areka の決め（SSP の実際に合わせる）。
  - **v0.1.0 へ移した**（版に縛らない列から）。
- 触るファイル: 棚卸㉓・㉔のまま（`catalog.rs` 453・`catalog_roots.rs` 252・`boot_resolve.rs` 676・`boot_config.rs` 467・`emo2_boot/shell_balloon_resolve.rs`・`menu/balloon_frame.rs`・`update/desk.rs`・`install/procedure.rs`・`readme/opener.rs`・記憶の形を変えるなら `crates/areka-sylphya/src/persist/`）。`areka-nar` は触らない見込み。
- 規模: 14〜20 タスク（探し方の段を足す分で上へ）。20 を超えそうなら「ゴーストの中を探して起動で使う」（ぱすたに要る分）と「メニュー・記憶・切り替え」に分ける案があるが、本文 Constraints の「起動はできるが、メニューに無く…バルーンを作らない」と食い違う＝分けるかは開発者が決める。
- 先に要るもの: なし（前の 3 本は着地済み）。重なり: `startup-options-core`（新・`boot_resolve.rs`・`boot_config.rs`）・`dev-folder-alias`（`catalog.rs`・`catalog_roots.rs`・`boot_resolve.rs`）・`shell-companion-balloon`・`config-parse-diagnostics`・`install-live-target-hazards`・`default-balloon-selfmade`（行き先待ち）。
- 優先度の区分: A のまま。要件定義のモデル: Fable のまま（ukadoc に無い独自の拡張・探し方と記憶と優先の骨格の分かれ目）。SSP の実測で探し方が決まれば Opus に下げてよい。
- 議題を足す（本文の 6 つの前に）: 0. **SSP で測る**（最初）: ぱすたを SSP 2.9.18 に入れたとき、`pasta_tategaki` をどう見つけるか（使っているバルーンの名前・置き場のプロパティ）・ゴーストの直下の他のフォルダも探すのか・バルーンのメニューに出るか・ほかのゴーストから見えるか。
- 古くなった記述: 本文 Desired Outcome の例の置き場（`<ゴースト>/balloon/<名>`）に、ぱすたの形（ゴーストの直下の名前の違うフォルダ）を足す。

## 2026-10-22 棚卸㉘の再測定（main 52b014de）
- 前提の変化: 鎖で前に居た ✅ `startup-options-core`（10-19）が着地した。根の印（`crates/areka-ghost/src/catalog.rs` の `RootMark`）に「この起動のあいだだけの置き場」（`Temporary`）が足され、`boot_resolve.rs`・`boot_config.rs` も触られた＝置き場の印に種類を足す先例が 2 つ（書ける根・一時の置き場）になった。✅ `mcp-reload`（10-22）は、使っているバルーンを場所で控えて読み直す道（`crates/areka/src/emo2_boot/reload.rs` の `current_skin`）を足した＝場所で持つので、ゴーストの中のバルーンでも読み直せる見込み（要件の段で確かめる）。ゴーストの中は今も見ない。
- 触るファイル（行数は今）: `crates/areka-ghost/src/catalog.rs`（475）・`catalog_roots.rs`（270）・`crates/areka/src/boot_resolve.rs`（714）・`boot_config.rs`（610）・`emo2_boot/shell_balloon_resolve.rs`（222）・`menu/balloon_frame.rs`（45）・`update/desk.rs`（704）・`install/procedure.rs`（515・入れた後に `find_balloon` で引く）・`readme/opener.rs`（406・バルーンは自前の `balloon_candidates` で引く）・確かめに `emo2_boot/reload.rs`（775）・記憶の形を変えるなら `crates/areka-sylphya/src/persist/`・`doc/COMPAT_ARCHITECTURE.md` §8・`dist/README.txt`。
- 規模: 14〜20 タスクのまま。
- 先に要るもの: 働きの上ではなし。同じファイルを触る未完了（同じウェーブに置けない）: v0.1.0＝`default-balloon-selfmade`（`boot_resolve.rs`）・`readonly-mode`（`boot_resolve.rs` の書く関数・`update/desk.rs`・sylphya の `persist/`）・`startup-install-option`（`boot_config.rs`）・`property-name-case-fold`（sylphya の `persist/` を畳むときだけ）。版に縛らない・後の版＝`dev-folder-alias`・`shell-companion-balloon`・`config-parse-diagnostics`・`install-live-target-hazards`・`clippy-199-lints`・`multi-ghost-runtime`（余力の枠）。依存を足さない（1 本だけの席は取らない）。静かな机・前後の全体テストの比べは要らない。議題 0（SSP で測る）は開発者の機械の SSP が要る。
- 優先度の区分: A（開発者「shell フォルダがあるんだから balloon フォルダがあっても良い」「別 spec に」）。
- 要件定義のモデル: **Opus に下げる**（厳しく付け直した）。置き場の印は先例（書ける根・一時の置き場）に種類を 1 つ足す形で、新しい境目にならない。並行や時刻の保証も無い。骨格の分かれ目（ゴーストの中をどう探すか）は議題 0 の SSP の実測で決まり、同じ名前のときの優先と記憶はシェル（`<ゴースト>/shell/`）の先例に倣える。測った結果が先例の無い形（ほかのゴーストからも見える・ゴーストの直下を全部探す、など）なら、そこで考え直す。
- 分割の案: なし（20 を超えるときの案は棚卸㉗のまま・本文 Constraints と食い違うので開発者が決める）。
- 見つけた穴・古くなった記述:
  - 本文 Constraints の「Fable 推奨」と棚卸㉓〜㉗の節の「Fable」は古い（上）。
  - 棚卸㉗の節の重なりの `startup-options-core` は着地済み。本文 Upstream の `ghost-standard-balloon` は着地済み、`shell-companion-balloon` は後ろ（前に居ると読める書き方は古い）。
  - バルーンの読み直し（`\![reload,balloon]`・MCP の `reload`）でゴーストの中のバルーンが保たれることを、確かめる項目に足す。

## 本 spec からの申し送り（2026-10-23・`areka-P0-pasta-in-windows-verification` のタスク 6）

- 出どころ: `areka-P0-pasta-in-windows-verification` の検証の記録 `verification.md`（5 節の崩れた所の表。件の番号は表の番号）と、SSP の測りの記録 `ssp-measurement.md`（SSP 2.9.23・書庫の sha256 `ad473d5d…a43a6`＝`pasta.toml` 1.0.0）。撮った物と記録は git の外の `<w>\target\pasta-verify\`（本走行 run1。`logs\`・`shots\run1\`・SSP の撮った物は `ssp\`）・`<w>\target\pasta-verify-alt\`（補いの走り alt2。縦書きのバルーンを根の `balloon\` へ写した置き方）・`<w>\target\pasta-verify-rerun\`（直しの後の回し直し rerun1。alt2 と同じ置き方）に在る（`<w>` は `C:\home\maz\git\areka\.claude\worktrees\areka-p0-pasta-windows-d4ed7a`）。記録の時刻は世界標準時（`Z`）のまま。日本時間は 9 時間足す。
- 開発者の決め（2026-10-23）「原則として、すべての申し送りが0.1.0に解決するのであれば承認する。申し送り漏れはダメですよ。」。この spec は v0.1.0 なので、下の件はこの spec の中で片づける。
- 着地した後の確かめは、ぱすたの手順書 `doc/pasta-real-run.md`（5 節の 7 段の表）で回す。
- **件 1（縦書きのバルーンが見つからない）**: run1 で、ぱすたの descript の `balloon` の名前 `窓際のぱすたさん縦書き` が見つからず、既定の横書きのバルーン `StayseeBalloon` で出た。
  - 根拠: `logs\run1.stdout.log` の `2026-10-22T23:17:53.448361Z` の `boot_resolve` の WARN `descript_balloon_not_found value="窓際のぱすたさん縦書き"`。get_log の `#3 … balloon_resolved route=Default dir=...\root\balloon\StayseeBalloon`（`shots\run1\1-boot-getlog.txt`）。
  - 原因: `crates/areka/src/boot_resolve.rs:294`〜`297`。descript の `balloon` の名前を、根の `balloon\` の一覧（`listed`）の中だけで探し、ゴーストのフォルダの中（ぱすたは `ghost\pasta-in-windows\balloon\pasta_tategaki`）を探さない。
- **件 2（横書きのバルーンで段が重なる）**: 件 1 の結果。横書きのバルーンでは、`\_l[-283,0]` の後の段が前の行に重なって描かれた（起動の段・`縦書き試作` の 2 ページとも）。`\_l[-253,0]` の後の選択肢 4 つも本文の 2 行目に重なった。ぱすたは縦書きの左の枠へ移るために y を 0 にしているので、横書きでは 1 行目の高さに戻る。
  - 根拠: `shots\run1\1-boot-balloon0.png`（「あれフ映ってるボール」）・`2-choice-zorder.png`・`2-choice-balloon0.png`・`2-tategaki-frames-page1.png`・`2-tategaki-balloon0-end.png`。
- **縦書きのバルーンが見つかれば崩れは消えること**: 縦書きのバルーンを根の `balloon\` へ写した alt2 と rerun1 では、`balloon_resolved route=Descript dir=...\root\balloon\pasta_tategaki` になり、`descript_balloon_not_found` は 0、列と枠は重ならなかった（`<w>\target\pasta-verify-alt\shots\alt2\1-boot-balloon0.png`・`2-choice-zorder.png`・`2-tategaki-burst-03.png`、`<w>\target\pasta-verify-rerun\shots\rerun1\2-tategaki-burst-03.png`・`2-tategaki-burst-09.png`）。この spec の着地で変わるのは見つけ方だけで足りる見込み。
- **SSP の基準・測りの答え**（`ssp-measurement.md` 2 節の段 1・段 2、3 節の 1 行目）: SSP は、ゴーストのフォルダの中の `pasta_tategaki`（`name,窓際のぱすたさん縦書き`）を使った（`get_property` のバルーンの名前・`ssp\stage1-shot.png`）。SSP の `balloon\` に同じバルーンが在るかは確かめていない。この見つけ方を書いた ukadoc の項目は見つからなかった。段 2 では `\_l[-253,0]`・`\_l[-283,0]` で左の枠へ移り、列と枠が崩れずに並んだ（`ssp\stage2-choice-shot.png`・`stage2-tategaki-frames-shot.png`）。
- **着地で変わるはずの所**（手順書の段 1・段 2）: バルーンが既定のバルーンから縦書きのバルーンに替わり、会話が縦書きで出る。
- 縦書きのバルーンの descript の `font.name`・`font.shadowcolor.*`・`cursor.*`・`anchor.font.color.*`・`vertical`・`use_self_alpha` が SSP と同じに見えるかは、開発者の判定がまだ無い（`verification.md` 6 節の「申し送りの漏れが無いことの確かめ」）。この spec の着地の後の回し直しで見比べる。`balloon-scroll-arrows`・`balloon-communicatebox` の実機の確かめも、この spec の後になる。
