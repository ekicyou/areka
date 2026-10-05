# SSP と areka の `dump_surface`・`dump_balloon` の差（2026-10-04 時点）

キャラクターの絵を返す `dump_surface` と、バルーンの絵を返す `dump_balloon` について、SSP と areka で答えが違う場合、または SSP の答えをまだ測っていない場合を行ごとに並べる。成功の本文と、無いスコープ・無い surface ID の文言のように SSP と同じものは載せない。輸送の差は [transport-diff-areka.md](transport-diff-areka.md) にある。

- **SSP の印**: 「実測」は SSP 2.9.07 の MCP で 2026-10-04 に測ったもの（ゴースト「えも2DEBUG」）。測った結果の正本は `.kiro/specs/areka-P0-mcp-dump-images/research.md` の「SSP の実測の追補」の節。「未実測」は SSP で測っていないもの。「対応物なし」は SSP にその仕組みが無いもの
- **areka の根拠**: どの文言を返すかを決める関数は `crates/areka/src/mcp/dump_surface_judge.rs`（文言の定数と、窓→スコープ→surface ID→一度も表示していない、の順の判断）。絵を集める処理は `crates/areka/src/mcp/dump_surface.rs` と `crates/areka/src/mcp/dump_balloon.rs`、バルーンの文字を重ねる処理は `crates/areka/src/mcp/dump_balloon_overlay.rs`

| 項目 | areka | SSP | SSP の印 |
|---|---|---|---|
| 一度も表示していないスコープで `surface` を省略 | `NG:No surface has been shown in this scope yet`・`isError: true`。返す絵が無いので areka の文言を置いた（`\s[-1]` で隠しているだけなら、隠す直前の絵と surface ID で成功する。これは SSP と同じ） | 隠した後は隠す直前の絵で成功する（実測）。起動してから一度も表示していないスコープは測っていない | 未実測 |
| 窓の無いゴースト（窓への結線ができず、記録だけの起動になった） | 2 本とも `NG:This ghost has no window`・`isError: true`（areka の文言） | 対応する状態が無い見込み | 未実測 |
| 一度も表示していないスコープで `surface` を指定 | 成功する。着せ替えは掛けない（表示の層に、最後に表示したときの着せ替えの記録がまだ無い。起動時にオンの着せ替えも掛からない） | 測ったゴーストに着せ替えが無く、測れなかった | 未実測 |
| 隠している間に着せ替えを変えた後の `surface` 指定 | 隠す前の着せ替えで返す。隠している間の `\![bind]` は着せ替えの状態だけを変えて表示の指令を出さないので、新しい着せ替えは次に表示するまで表示の層に届かない | 測っていない | 未実測 |
| 画面の拡大率が 1 でないときのバルーンの字 | 絵は原寸で返り、背景と字の位置はずれない。ただし字は、拡大した文字の面を原寸へ縮めて（面積で重み付けした平均）背景へ重ねるので、輪郭が少しにじむ（200% ではほぼ劣化しない・125%・150% は少しにじむ） | SSP の拡大の設定を変えて測っていない | 未実測 |
| 負のスコープ（`scope: -1` など） | 2 本とも `NG:No such scope in this ghost`・`isError: true`（ツールの結果として返る） | JSON-RPC のエラー `Invalid params`（`NG:` のツールの結果ではない） | 実測 |
| シェルの絵の中の箱（`\b[名前]`）に書いた文字 | `dump_surface` に写らない（画面では箱に字が見えていても、返る絵は字の無いキャラクターの絵）。`dump_balloon` にも含めない（普通のバルーンの文字の面だけを読む） | 箱は areka の拡張で、SSP に同じ仕組みが無い | 対応物なし |
| バルーンの窓の印（上へ送る矢印・SSTP の送り主の印） | areka が描いているものだけが写る。今の areka はバルーンの背景を組むときにスクロールの矢印（`arrow*`）とマーカー（`marker*`）の絵を読まず、送り主の印も描いていないので、どちらも写らない（背景と字だけ） | 背景・字のほかに、上へ送る矢印と送り主の印（例 `from MCP (local)`）が写る | 実測 |

## 補足

- areka のバルーンの背景が `arrow*`・`marker*` を読まないことは、バルーンの絵を組む処理（`crates/areka-emo-present/src/balloon.rs` の冒頭の説明）に書いてある。印を描くようになれば、`dump_balloon` は手を入れずにそれも写す。
- 文言は `dump_surface_judge.rs` の決定論テスト（`dump_surface_judge_tests.rs`）が固定している。文言を変えたら、この表も直す。
