# Brief: areka-P0-extra-character-windows

> 2026-10-04 棚卸㉑で起票（roadmap「覚え書き」の「3 人目以降のキャラクターの窓」を格上げ）。**段はその他**。

## Problem

- **利用者**: キャラクターが 3 人以上いるゴーストを入れると、3 人目からの窓が出ない。台詞の `\p[2]` 以降は誰の口からも出ない（説明書 `dist/README.txt` の「既知の制限」に「3 人目からの窓は出ません」と書いてある）。
- **ゴースト作者**: 正典は 3 人目以降のスコープを当たり前に使う。ukadoc の `\p[ID番号]`（https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5cp_5bID_756a_53f7_5d:1）の記述例は「\p[2]3人めが喋る。\p[3]4人めが喋る。」。
- **開発者**: 持ち主の spec が居ない。未着手の `popup-menu-residue` の brief の項目 9 は「`char{n}`（n≧2）を `ghost-shell-balloon-switch` 以降の多キャラクター対応へ」と書いたが、渡し先は完了済みで、今どの spec も持っていない。

## Current State

- 窓の数を決める所が 0 と 1 に固定されている: `crates/areka/src/emo2_boot/mod.rs` の `derive_scopes()` は `vec![0, 1]` を返すだけ（コメントは「M1 受容トレードオフ」）。
- 3 人目を想定しない検査が 2 か所ある: `crates/areka/src/input_events/mod.rs` の `char_scope` と、`crates/areka/src/menu/captions.rs` の n≧2 の扱い（どちらも `debug_assert!(scope <= 1, …)`）。メニュー側は `UNQUERIED_POPUPMENU_RESOURCES` に `char*.popupmenu.visible`（n≧2）を「聞かない資源」として登記してある。
- 既にできている部分:
  - placement: `crates/areka/src/placement/config.rs` の `detect_scopes`・`char_scope_of` が `char{n}.` 接頭のキーから scope n を見つける。`placement/source.rs` は `char{n}.name` を窓の題に使う。`placement/measure.rs` も scope≧2 の分岐を持つ。
  - バルーン: `crates/areka-emo-present/src/balloon.rs` は scope n（n≧2）の流用の連鎖（`balloonp{n}def` → `balloonk` → …）を正典どおりに持つ。
  - 台本: `crates/areka-parsers/src/sakura/decode.rs` は `\p[n]` と `\pN` を `SpeakerScope{n}` へ写す。
- つまり、読む側と描く側の部品は n≧2 を知っているのに、窓を作る入口が 2 つに絞っている。

## Desired Outcome

- `char{n}.*`（n≧2）を持つゴーストで、その数だけキャラクターの窓とバルーンの窓が立ち、`\p[n]` の台詞がその窓のバルーンに出る。
- 3 人目以降の窓も、ドラッグ・位置の記憶・右クリックメニュー・当たり判定が 1 人目・2 人目と同じに動く。
- 説明書の「3 人目からの窓は出ません」を消せる。

## Approach

- 窓の数の導き方を 1 か所にする。今は emo2_boot の `derive_scopes` と placement の `detect_scopes` が別々に導いている（`derive_scopes` のコメントが「導出の二元性」と書いている）。placement の実結果を起動の側へ渡して、同じ答えを使う形を設計で決める。
- 2 人までを前提にした検査（`debug_assert!`）を外し、n≧2 の分岐を実際に通るテストで押さえる。
- メニューの `char*.popupmenu.visible`（n≧2）を、3 人目以降の窓が出るのに合わせて聞く側へ移す。

## Scope

- **In**:
  - 窓の数の導き方の一本化（emo2_boot と placement）。
  - キャラクター窓・バルーン窓の生成と片付け（ゴーストの切替と終了を含む）。
  - `input_events` のスコープの扱い・右クリックメニューの n≧2（`char*.popupmenu.visible`）。
  - 位置の記憶（`placement/persist.rs`）と復元が n≧2 で効くこと。
  - 決定論のテスト（偽の境界で 3 人構成を組む）と、実機の確かめ（3 人以上の検体）。
  - 網羅台帳 `doc/ukadoc-coverage/ledger/shiori.toml` の `char*.popupmenu.visible` の行と、`dist/README.txt` の既知の制限の 1 行。
- **Out**:
  - 複数のゴーストを同時に出すこと（roadmap「予約」の多重ゴースト）。
  - `\![set,sticky-window,…]`・`\![set,zorder,…]` の 3 人目以降の組み合わせ（窓の重なり順の spec の領分。窓が出た後に効くかは確かめるだけ）。
  - `*.popupmenu.type` の「省略した形」（正典が中身を定めていない・`popup-menu-residue` の項目 9 の残り）。

## Boundary Candidates

- 窓の数を導く 1 か所（placement の答えを起動の側へ運ぶ形）。
- 窓を作る・消す入口（emo2_boot の結線）。
- ポインタとメニューのスコープの扱い（input_events・menu）。

## Out of Boundary

- 合成と文字の層の中身（emo-present・emo-text）は n を引数に受けるだけで、本 spec は変えない見込み。変える必要が出たら止めて報告する。
- 窓どうしの重なり順の仕組み（`zorder-chain-residue`・`zorder-property`）。

## Upstream / Downstream

- **Upstream**: 完了 `window-placement`（`detect_scopes`）・完了 `emo2-boot`・完了 `kero-balloon`（scope≧2 のバルーンの流用）・完了 `popup-menu-minimal`。
- **Downstream**: `popup-menu-residue`（項目 9 の `char{n}` の部分を本 spec が引き取る）・`zorder-property`（3 人目以降の窓を重なり順の対象に含める）。

## Existing Spec Touchpoints

- **Extends**: 完了 `emo2-boot` の `derive_scopes`（M1 の 2 スコープ固定）と、完了 `window-placement` の `detect_scopes`。
- **Adjacent**: `popup-menu-residue`（`menu/captions.rs`）・`shell-balloon-frame-align`／`balloon-canon-residue`／`shell-companion-balloon`（どれも `emo2_boot` を触る）・`mouse-drag-events`（`input_events/mod.rs`）。

## Constraints

- 1 ファイル 1,000 行以下。`emo2_boot/mod.rs` は 870 行台なので、足す分は兄弟ファイルへ出す。
- テストは実装と同じフォルダの兄弟ファイルへ。常時のテストは x64 と偽の境界で回す。
- ログの無い失敗の経路を作らない。実機の確かめの根・検体・一時フォルダはワークツリーの `target\` の下だけ。

## 2026-10-04 棚卸㉑で起票

- **出どころ**: roadmap「覚え書き」の「3 人目以降のキャラクターの窓」（10-02 登記）。説明書の既知の制限の 1 行も同じ件。
- **規模**: M〜L（14〜20 タスク）。20 を超えそうなら、メニューの n≧2 を `popup-menu-residue` へ戻す。
- **前提**: `emo2_boot` の列の順番待ち＝`shell-balloon-frame-align` の後、`balloon-canon-residue`・`shell-companion-balloon` と触るファイルを照合してから並べる。
- **触るファイル**: `crates/areka/src/emo2_boot/{mod.rs, frame/attach.rs}` と兄弟・`crates/areka/src/input_events/mod.rs`・`crates/areka/src/menu/captions.rs`・`crates/areka/src/placement/{mod,config,source,measure,spawn,persist}.rs`・`doc/ukadoc-coverage/ledger/shiori.toml`・`dist/README.txt`。
- **共有しうる相手**: `emo2_boot` を触る spec 全般・`mouse-drag-events`（`input_events/mod.rs`）・`popup-menu-residue`（`menu/`）・`dist/README.txt` を触る配布の spec。
- **議題**:
  1. 窓の数を何から導くか（ゴーストの descript の `char{n}.*`／シェルの descript の `char{n}.*`／サーフェスの有無）。今の `detect_scopes` は ghost と shell の両方の `char{n}.` を見る。
  2. 3 人以上いる検体の入手（再配布の条件の明確なもの・`vendors/sample_ghost/` の決まりに従う）。
  3. `popup-menu-residue` の項目 9 のうち `char*.popupmenu.visible`（n≧2）を本 spec で引き取るか。


## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模: M〜L（14〜20 タスク）。変わらず。切らない。
- 前提の状態: 未。`emo2_boot` の結線の列で `balloon-font-file`（C4）→ `balloon-canon-residue` ∥ `shell-companion-balloon` の後。加えて、新しく起票されたバグ `char-position-save-on-exit` が同じ位置の記憶（`placement/persist.rs`・`placement/follow/drag_follow.rs`）を触る＝あちらが先に着地すれば、3 人目以降の位置の記憶も「終了で書く」の形の上に載る。
- 崩れた前提／古くなった位置:
  - 窓の数を 0・1 に固定する `derive_scopes`（`emo2_boot/mod.rs`）の呼び手は 2 か所＝起動の手順 1 と、シェル・バルーンの着替えの `emo2_boot/switch_assets.rs`。**後者が触るファイルに漏れていた**。
  - `emo2_boot/mod.rs` は 883 行（起票時の 870 台から増えた）。足す分は兄弟ファイルへ出す方針のまま。
  - `mouse-drag-events` が `input_events/drag.rs`（ドラッグの知らせ）を足した。スコープは `char_scope` から取るので、`char_scope` の `debug_assert!(scope <= 1, …)` を外せば 3 人目のドラッグも通る＝`drag.rs` 自体は触らない見込み。メニューの側の `debug_assert!`（`menu/captions.rs`）も起票時のまま。
  - C3 で `placement/` に入ったのは `#[allow(dead_code)]` の注記の外しだけ（`config.rs`・`measure.rs`・`persist.rs`・`source.rs`・`spawn.rs` ほか）。`detect_scopes`・`char_scope_of` の形は変わらない。
- 触るファイル（並走の照合用）:
  - `crates/areka/src/emo2_boot/{mod.rs, switch_assets.rs, frame/attach.rs}` と兄弟
  - `crates/areka/src/input_events/mod.rs`・`crates/areka/src/menu/captions.rs`
  - `crates/areka/src/placement/{mod,config,source,measure,spawn,persist}.rs`
  - `doc/ukadoc-coverage/ledger/shiori.toml`・`dist/README.txt`
  - 共有しうる相手（増えた分）: `placement-measure-bake-once` の案 A（`placement/mod.rs`・`measure.rs`・`emo2_boot/mod.rs`）・`char-position-save-on-exit`（`placement/persist.rs`）。
- 議題（答えで作業が変わるものだけ）: 起票時の 3 つのまま。
- 見つけた穴: 触るファイルの一覧に `emo2_boot/switch_assets.rs` が無かった（上）。


## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- 前提の変化:
  - `char-position-save-on-exit` が着地した（10-08）。位置の記憶は「並べ終えた時点で、記憶に無い窓の位置を書く」形になった（`crates/areka/src/placement/persist.rs`・642 行）。書く関数はスコープの番号を引数に取るので、3 人目以降もそのまま載る見込み。
  - 窓の数を 0・1 に固定する `derive_scopes`（`crates/areka/src/emo2_boot/mod.rs`）と呼び手 2 か所（起動・`emo2_boot/switch_assets.rs`）、2 人までを前提にした検査 2 か所（`input_events/mod.rs` の `char_scope`・`menu/captions.rs`）は起票のまま。
  - 3 人いる検体が手に入った: `vendors/sample_ghost/emily4.nar`（`char2.name,Emilio`・`char2.seriko.defaultsurface,200`。CC BY-NC 4.0 なので配布物には入れない）。検体の登記は `emily-ghost-verification` が行う＝議題 2 はそちらの着地で解ける。
- 触るファイル: `crates/areka/src/emo2_boot/{mod.rs, switch_assets.rs, assets.rs, frame/attach.rs}` と兄弟・`input_events/mod.rs`（571）・`menu/captions.rs`（302）・`placement/{mod,config,source,measure,spawn,persist}.rs`・台帳 `shiori.toml`・`dist/README.txt` の「3 人目からの窓は出ません」の行。
- 規模: 14〜20 タスク（変わらず）。
- 先に要るもの: 働きの上では無し（検体は `emily-ghost-verification` の登記の後だと楽）。ファイルの重なりは次のとおり。
  - `balloon-canon-residue`（`frame/attach.rs`・`placement/config.rs`）・`balloon-font-file`（`frame/attach.rs`）。
  - `mcp-reload`（`emo2_boot/mod.rs`）・`placement-measure-bake-once`（`emo2_boot/mod.rs`・`placement/{mod,measure}.rs`）・`dpi-realign-remembered-chain`（`placement/`・着手の前に照合）。
  - `popup-menu-residue`（`menu/captions.rs`）・`shell-companion-balloon`（`switch_assets.rs`・向こうが「一度に替える」を選んだ場合だけ）。
- 優先度の区分: C（ukadoc の `\p[2]` 以降の拾い残し。えみりは 3 人なので、`emily-ghost-verification` の結果しだいで開発者の依頼の続きになりうる）。
- 要件定義のモデル: Fable（窓の数の導き方を、起動・配置・入力・メニューの 4 か所で揃える）。
- 分割の案: なし（20 を超えそうなら、起票のとおりメニューの n≧2 を `popup-menu-residue` へ戻す）。
- 見つけた穴・古くなった記述:
  - 触るファイルの一覧に `emo2_boot/assets.rs` が無かった。最初に出す面の番号を「スコープ 0 なら 0・それ以外は 10」の決め打ちで決めている所で、descript の `char*.seriko.defaultsurface`（`sakura.`・`kero.` も）はどこも読んでいない。同じ 10 の決め打ちは `placement/measure.rs` にも在る。
  - `emo2_boot/mod.rs` は 912 行・`placement/mod.rs` は 902 行＝足す分は兄弟ファイルへ出す。
