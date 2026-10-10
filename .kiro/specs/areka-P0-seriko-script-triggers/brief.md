# Brief: areka-P0-seriko-script-triggers

> 2026-10-10 棚卸㉓で `areka-P0-seriko-trigger-intervals` から切り出した。同 spec が 22〜27 タスクで上限の 20 を超え、しかもこの部分だけが台本の読み手・台本のコンパイル・運び手の名前の表という別の列のファイルを触るため。サーフェスアニメーションのテーマの 2 本目（1 本目は口パクの `seriko-trigger-intervals`）。

## Problem

- **ゴーストの作者**: 台本から呼ぶアニメーションが動かない。
  - `yen-e`（台本の `\e` が来たときに再生する）が動かない。
  - 台本のタグ `\i[ID番号]`（今のスコープの側で、指した番号のアニメーションを表示する）が効かない。
  - `never`（自動では動かず、`\i` などで呼ばれたときだけ再生する）は、呼ぶ手段が無いので一度も動かない。
  - `animation*.name` で付けた名前を、`\i` の ID の代わりに使えない。
- **開発者**: 持ち主の spec が居なかった。網羅台帳で 4 行とも担当なし。

正典の出どころ（ukadoc の文書 MCP で引いた見出し。文は要約）:

- `yen-e`（descript_shell_surfaces・https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#yen-e ）: 台本で `\e` が来たときに実行する。
- `never`（同・https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#never ）: 自動では実行しない。ほかのアニメーションからの `start`・`alternativestart` などの呼び出しか、台本の `\i[*]` などの命令でだけ再生する。
- `\i[ID番号]`（list_sakura_script・https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_i_ID%E7%95%AA%E5%8F%B7_ ）: 今のスコープの側に、その番号のサーフェスアニメーションを表示する。`animation*.name` で定義した文字列を ID の代わりに使える。
- `animation*.name,定義名`（descript_shell_surfaces・https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#animation*.name%2C%E5%AE%9A%E7%BE%A9%E5%90%8D ・SSP 2.8.24）: アニメーションの名前。番号と同じように、`\i[]`・`\![anim]`・`start`・`stop` などの ID の欄に使える。

## Current State

今の木（main `ee3af616`）で確かめたこと。

- 台本の読み手: `crates/areka-parsers/src/sakura/decode.rs` は `\i` を「subset 外の正準タグ」の腕（`decode_passthrough_tag`）で、「知らないタグ」の印を付けた素通しの `Raw` にする。
- 台本のコンパイル: `crates/areka-sakura/src/compile.rs` は、`\e` の腕（`Instruction::End`）で終わりの理由を決めて走査を打ち切るだけで、cue を出さない。`Raw` は控えめな記録を残して捨てる。
- seriko: 全部の cue を受け取っている。汎用の運び手は、cue の受け口（`crates/areka-seriko/src/actor.rs` の `handle_message`）が名前 `bind` だけを開封し、ほかの名前は読み飛ばす。
- 運び手の名前の表: `crates/areka/src/emo2_boot/consumer_ledger.rs` の `canonical`（ファイルは 943 行＝上限 1,000 の近く）。表と受け手の選別がずれないことを、兄弟のテスト `consumer_ledger_agreement_tests.rs` が確かめる。MCP の `check_script`（`crates/areka/src/mcp/check_script*.rs`）はこの表を引いて「知らない `\!`」を答える。
- シェルの読み手: `crates/areka-parsers/src/shell/decode.rs` の `decode_animations` が値にするのは、接尾辞が `interval` の行と `pattern` で始まる行だけ。`animation*.name` の行は黙って落ちる（記録なし）。型 `Animation`（同じフォルダの `model.rs`）の欄は `id`・`interval`・`patterns` の 3 つで、この型を直書きしているファイルは 23。
- seriko の表: `crates/areka-seriko/src/table.rs` は `yen-e`・`never` を `Interval::Other` の腕で、元の綴りを添えた控えめな記録を残して採らない。
- 網羅台帳: `yen-e`・`never` は `vocabulary-only`・担当なし、`animation*.name` は `absent`・担当なし（`doc/ukadoc-coverage/ledger/assets.toml`）。`\i[ID番号]` は `absent`・担当なし（`sakura-script.toml`）。

## Desired Outcome

- `yen-e` のアニメーションが、台本の `\e` で 1 回再生される。
- `\i[ID番号]` で、今のスコープの今のサーフェスにある、その番号のアニメーションが 1 回再生される。番号の代わりに `animation*.name` の名前でも呼べる。
- `never` のアニメーションは自動では動かず、`\i` で呼ばれたときだけ動く。
- 指した番号・名前が今のサーフェスに無いときは、記録を残して何もしない（黙って捨てる経路を作らない）。

## Approach

- 台本の読み手で `\i` を腕にし、コンパイルが汎用の運び手（`\!` コマンドと同じ 1 本の口）へ載せる。新しい個別の型は作らない（決まり「`\!` コマンドは汎用の運び手 1 本」）。
- `\e` は、コンパイルの終わりの腕が、台本を切り詰める前に「台本の終わり」の知らせを同じ運び手で 1 つ出す。
- seriko は、受け口で自分宛ての名前（今は `bind` だけ）に 2 つを足して開封する。運び手の名前の表に同じ 2 つを登録し、一致のテストと `check_script` の表を揃える。
- シェルの読み手が `animation*.name` を落とさずに運ぶ。型 `Animation` に欄を足すと直書きの 23 ファイルが壊れるので、欄を足す形と、面ごとの別の表で運ぶ形を設計で比べる。
- 再生は、`seriko-trigger-intervals` が入れる「引き金が来たら 1 回流す」の形にそのまま乗せる。時計を増やさない。

## Scope

- **In**:
  - interval の `yen-e`・`never`。
  - 台本のタグ `\i[ID番号]`（番号と、`animation*.name` の名前）。
  - `animation*.name` の読み。
  - 決定論のテスト・運び手の名前の表の登録と一致のテスト・網羅台帳の 4 行・実機の確かめ（`\i` と `yen-e` を持つ検体。`seriko-trigger-intervals` の検体に足せるなら足す）。
- **Out**（別途）:
  - `\i[ID,wait]`・`\__w[animation,ID]`（アニメーションの終わりまで台詞を待たせる＝台本の進行との同期が要る）。
  - `\![anim,…]` 系のタグ。`start`・`stop`・`alternativestart` などの、アニメーションから別のアニメーションを呼ぶメソッド。`animation*.name` の名前はこれらの ID の欄にも使えると正典は書くが、タグとメソッドの側が無いので、この spec は `\i` で使う所までを持つ。
  - `talk`・`runonce`・`periodic`（`seriko-trigger-intervals`）。`+` の組み合わせ（`seriko-interval-combinations`）。

## Boundary Candidates

- 台本の読み手の `\i` の腕と、コンパイルの `\e` の知らせ（`areka-parsers` の `sakura/`・`areka-sakura` の `compile.rs`）。
- 運び手の名前の表（`consumer_ledger.rs` と一致のテスト）。
- シェルの読み手の `animation*.name`（`shell/{model,decode}.rs`）。
- seriko の表と受け口（`table.rs`・`actor.rs`・`looper.rs`）。

## Out of Boundary

- 台本の進行そのもの（kanade・dola の cue）の作り替え。知らせを運び手に載せる以上は変えない。
- 合成と描画（emo-compose・emo-present）。

## Upstream / Downstream

- **Upstream**: `seriko-trigger-intervals`（「引き金が来たら 1 回流す」の形と、seriko の同じファイル）。台本のコンパイルの列では `anchor-tag-canon` の後。
- **Downstream**: `\i[ID,wait]`・`\![anim,…]`・`start` ほかを持つ次の spec（別途）。`seriko-interval-combinations` とは seriko の `table.rs`・`looper.rs` を分け合うので直列。

## Existing Spec Touchpoints

- **Extends**: `seriko-trigger-intervals` の範囲から `yen-e`・`never`・`\i[ID番号]`・`animation*.name` を引き取る。
- **Adjacent**: `anchor-tag-canon`・`choice-ranges-one-function`・`balloon-font-file`・`choice-marker-styling`・`talk-fast-forward`・`balloon-markers`・`text-typesetting`（台本の読み手と `compile.rs`）。`mcp-author-tools`（完了。運び手の名前の表と `check_script` の表を揃えるテストを持つ）。読み手の `shell/{model,decode}.rs` を触る `draw-methods-canon`・`collisionex-regions`・`element-clipping-option`・`animated-image-import`。

## Constraints

- `consumer_ledger.rs` は 943 行。登録の行だけを足し、テストは兄弟のファイルへ置く。1,000 行を超えるなら先に分ける。
- 新しい運び手の名前は、必ず表に登録し、一致のテストに載せる（表に無い名前は `check_script` が「知らない」と答える）。
- 1 フレーム遅らせる解は取らない。`\e` の知らせと再生の始まりを同じフレームで解く。時刻は正確に扱う（待ち時間を丸めない）。
- アニメのエンジンは sakura と seriko の 2 つのまま。3 つ目の時計を作らない。
- テストは兄弟のファイルへ。1 ファイル 1,000 行以下。ログの無い失敗の経路を作らない。

## 2026-10-10 棚卸㉓の測定（main `ee3af616`）

- **触るファイル**:
  - `crates/areka-parsers/src/sakura/decode.rs`（477 行・`\i` の腕。命令の型を足すなら同じフォルダの型のファイルも）
  - `crates/areka-sakura/src/compile.rs`（411 行・`\e` の知らせと `\i` を運び手へ載せる）
  - `crates/areka/src/emo2_boot/consumer_ledger.rs`（943 行）と `consumer_ledger_agreement_tests.rs`、表を引く `crates/areka/src/mcp/check_script*.rs`（揃えるテストが赤になった分だけ）
  - `crates/areka-parsers/src/shell/{model.rs, decode.rs}`（`animation*.name`）
  - `crates/areka-seriko/src/{table,actor,looper}.rs` と兄弟のテスト
  - `doc/ukadoc-coverage/ledger/assets.toml` の 3 行（`yen-e`・`never`・`animation*.name`）と `sakura-script.toml` の 1 行（`\i[ID番号]`）
- **規模**: 8〜10 タスク。
- **先に要るもの**: `seriko-trigger-intervals`（働きと seriko のファイル）。ファイルの順として `anchor-tag-canon`（台本の読み手と `compile.rs`）の後。
- **優先度の区分**: A（開発者 10-05「口パクの `talk` が動かない」→ 優先度を高へ。切り出し元の区分を引き継ぐ）。
- **要件定義のモデル**: Fable（`\e` の知らせと再生を同じフレームで解く時刻の判断・正典が黙っている所の決め）。
- **議題**（答えで作業が変わるものだけ）:
  1. `\i` が指せるアニメーション。`never` だけか、interval が何であっても 1 回流すか。再生の途中でもう一度呼ばれたら頭からやり直すか。正典は「表示する」とだけ書く。
  2. `\e` を書かずに終わった台本と、途中で中断された台本で、`yen-e` を起こすか。正典は「`\e` が来た時」とだけ書く。
  3. `animation*.name` の名前が 2 つのアニメーションで重なったとき・数字だけの名前（番号と見分けが付かない）のとき、どちらを採るか。正典は黙っている。
