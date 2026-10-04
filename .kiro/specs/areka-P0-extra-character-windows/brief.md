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
