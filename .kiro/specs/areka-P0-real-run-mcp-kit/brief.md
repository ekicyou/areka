# Brief: areka-P0-real-run-mcp-kit

> 2026-10-18 棚卸㉗で起票（開発者の指示「0.1.0のスコープを決定しました。ゴールは、SSP並みの自動試験が出来るようにすること、とします。SSPなみの起動オプション、MCPサポートの安定化、kiro-watchの機能追加など、開発がスピードアップすることを第一目標とします」＝v0.1.0 のゴール）。根拠は棚卸㉗の調べ（MCP の安定化と開発・試験の道具の見直し・main `2d86dbff`）。区分 **A**。版 **v0.1.0**。SSP の正典に当たる頁は無い（試験の足場）。SSP の起動オプションとの関わりは SSP ヘルプ「起動オプション一覧」（ukadoc MCP の `ssphelp:option`）の `--sstp-listen`・`--option readonly` の「起動中の他の SSP と干渉しない試験用の SSP を立てる」。

## Problem

- **開発者・AI のエージェント**: areka を本物の窓・GPU・SHIORI で立て、MCP で台本を流し、記録と絵を読んで確かめる「実機の試験」を書くたびに、同じ足場を一から組んでいる。areka の実機の試験 3 本（`crates/areka/tests/emo2_real_run.rs`・`mcp_get_log_real_run.rs`・`smoke_boot_loop_exit.rs`）は、それぞれが自分で:
  - 検体の写しを取り、`areka.exe` の隣に 32bit の仲介（i686 の `shiori-host32-helper.exe`）を置く（同じ関数 `is_i686_pe`・`ensure_helper_beside_areka` が `mcp_get_log_real_run.rs` 57・71 行目と `smoke_boot_loop_exit.rs` 147・204 行目に 2 つずつ写してある）。
  - 環境変数の組を書く（`emo2_real_run.rs` は `AREKA_ROOT`・`AREKA_PROFILE_DIR` を外さず、`smoke_boot_loop_exit.rs` は外す・`mcp_get_log_real_run.rs` は `AREKA_PROFILE_DIR` を一時の置き場へ向ける＝3 本で組がばらばら）。
  - 子の出力を読む（`emo2_real_run.rs` はパイプ・`mcp_get_log_real_run.rs` はパイプが詰まるのを避けてファイル）。
  - 終わらせ方を自動終了（`AREKA_APP_SMOKE_EXIT_MS`）の残り時間に頼る（`emo2_real_run.rs` は 3 秒＝`emo2-real-run-stability` の件 1 の赤の原因の見立て）。
  - MCP を呼ぶ（`mcp_get_log_real_run.rs` は HTTP と JSON を手で書いている・`serde_json` も足していない）。
- 足場がばらばらなので、v0.1.0 で MCP のツールが揃っても（`mcp-kanade-tools`・`mcp-reload`）、それを使う実機の試験を書くたびに同じ穴を踏む。SSP なら `--option readonly --sstp-listen 9900` で「干渉しない試験用の SSP」を 1 行で立て、MCP で回せる。

## Current State

- 検体の写しは `sample-ghost-kit`（`crates/sample-ghost-kit/src/lib.rs` の `SampleRoot::acquire`）が窓口を 1 つに揃えている（使い捨ての写し・値を捨てると消える）。実機の起動・MCP の呼び出し・終わらせ方の窓口は無い。
- MCP の受け口はつなぐ先 `127.0.0.1`・番号は `AREKA_MCP_PORT`（`crates/areka-mcp/src/port.rs`）。待受を始めたことは記録の行に出る。
- MCP の公式 SDK（rmcp `=3.5.0`）のクライアントを、自前の HTTP の送り手で動かす先例が中継にある（`crates/areka-mcp-bridge/src/http.rs`＝rmcp の `StreamableHttpClient` を tokio の `TcpStream` と `httparse` で満たす・`lib.rs` で `pub mod http`）。
- 利用者の閉じ方と同じ終わらせ方（Alt+F4 と同じ道）は `crates/areka/src/app_exit.rs` の `on_ghost_os_close`（メニューの「終了」と同じ終了の要求を kanade へ送る → `OnClose` → 終了コード 0）。台本 `\-` で閉じる道は `mcp-kanade-tools`（`sakurascript`）の後に使える。
- 二重起動の確かめは無い（試験は areka を並べて立てることに頼る）。

## Desired Outcome

- 実機の試験を書く人が、1 つの窓口で次をできる:
  - 検体（`sample-ghost-kit` の名前）か任意のゴーストのフォルダを渡して、`target\` の下の一時の根・一時の記憶の置き場・空いた MCP の番号で `areka.exe` を立てる（32bit の仲介は自動で隣へ置く。開発者の環境変数 `AREKA_ROOT`・`AREKA_USER_ROOT`・`AREKA_PROFILE_DIR` は受け継がない）。
  - MCP のツールを名前と引数で呼び、答えの文を受け取る（`call("get_log", {...})`）。待受が立つまで待つのは記録の行の観測で行う。
  - 子の標準出力と標準エラーをファイルへ流し、走りながら文字列を探して待つ（有界の待ち・時間切れの文言にどこで待っていたかを書く）。
  - 終わらせるときは利用者と同じ道（`OnClose`）で閉じ、終了コードを返す。自動終了は保険の締切としてだけ使う。落ちたら、その回の根と出力を消さずに残す。
- 今の実機の試験 3 本がこの窓口へ移り、写した関数が 0 になる。試験の確かめの中身は変えない。
- 窓口は、`startup-options-core` が着地した後は環境変数の代わりに起動オプション（`--sstp-listen`・`--option readonly`）でも立てられる（後から差し替えられる作り）。

## Approach

- 窓口を 1 つのクレートの中の 1 つのモジュールに置く（置き場は議題 1）。中身は ⑴ 立てる（根・記憶の置き場・番号・仲介・環境変数）⑵ 出力をファイルへ流して探す ⑶ MCP を呼ぶ ⑷ 閉じる、の 4 つ。
- MCP の呼び出しは手で書かず、公式 SDK のクライアントを中継の HTTP の送り手で動かす（中継が先例・依存を増やさない）。手で HTTP を書く今の `mcp_get_log_real_run.rs` の形はやめる。
- 閉じ方は、子の窓へ OS の閉じる知らせを送る形（`on_ghost_os_close` の道）を推す。`emo2-real-run-stability` の件 1 の直し（印を見てから閉じる）と同じ仕組みなので、どちらが先に作っても同じものを使う（議題 2）。

## Scope

- **In**: 窓口（立てる・探す・呼ぶ・閉じる）・32bit の仲介の置き方の 1 本化・空いた番号の取り方・落ちた回の証拠の残し方・今の実機の試験 3 本の移し替え・窓口そのものの決定論のテスト（偽の子と偽の受け口で）・使い方の説明（窓口の doc と `doc/` の試験の手引きの 1 節）。
- **Out**: 新しい実機の試験を足すこと（各 spec が窓口を使って足す）・ぱすたの検体の取り寄せ（`sample-url-fetch`）・MCP のツールの中身（`mcp-kanade-tools`・`mcp-reload` ほか）・起動オプション（`startup-options-core`）・机の取り合い（`impl-watch-test-desk`）・画面の入力（`SendInput`・computer-use）の足場。

## Boundary Candidates

- 窓口（試験の側）と、areka 本体の起動の約束（環境変数・起動オプション・記録の行）の境。本体の側は変えない見込み。
- MCP の呼び出し（公式 SDK＋中継の送り手）と、答えの読み方（文の取り出し）。
- 閉じ方（OS の閉じる知らせ）と保険の自動終了。

## Out of Boundary

- `areka.exe` の本番の振る舞い（終了の握手・MCP の受け口）。
- `sample-ghost-kit` の検体の展開と名前の窓口（使うだけ）。

## Upstream / Downstream

- **Upstream**: なし（今ある足場の寄せ集め）。`emo2-real-run-stability` の件 1 と閉じ方が同じ（議題 2）。
- **Downstream**: `mcp-kanade-tools`・`mcp-reload`・`mcp-strict-errors`・`mcp-receiver-bugs` の件 2（実機の確かめ）・`pasta-in-windows-verification`（ぱすたを MCP で回す）・`startup-install-option`（入れて確かめる）・`emo2-real-run-stability` の件 2（数十回のくり返し）。

## Existing Spec Touchpoints

- **Extends**: なし。完了 `nar-install`（`sample-ghost-kit` の窓口）・完了 `mcp-stdio-bridge`（中継の HTTP の送り手）の上に立つ。
- **Adjacent**: `emo2-real-run-stability`（`emo2_real_run.rs` を本 spec が移し、向こうが待ち方を直す＝同じファイル・同じ閉じ方）・`test-roots-under-target`（一時の根の置き場の決まり）・`sample-url-fetch`・`default-balloon-selfmade`・`dot-sakura-specimen`（`crates/sample-ghost-kit/src/lib.rs` を触る）・`impl-watch-test-desk`（実機の試験は排他テストの机を取って回す）・`real-run-log-noise`（実機の走りの記録の雑音）。

## Constraints

- 試験の根・検体・一時フォルダ・出力はワークツリーの `target\` の下だけ（Windows 形の絶対パス・短いパス＝emo2 の実走は絶対パスかつ短いパス）。
- 止めてよいプロセスは、窓口が自分で起こしたと確かめられる子だけ。
- 標準の通信の決まり（MCP）は手で書かない＝公式 SDK を使う。tokio は呼び出しの中に閉じ込める。
- 待ちは観測で（印が出たら進む・有界）。sleep を足す・締切を延ばすだけ・1 フレーム遅らせる形では作らない。GPU は実の GPU。
- 実機の窓が出るので、負荷の走り（`areka-test-binary-stability`）と同じ時間に回さない。

## 2026-10-18 棚卸㉗の見立て（main 2d86dbff）

- 触るファイル: 窓口の新しいモジュール（案 ⑴ `crates/sample-ghost-kit/src/run.rs`＋兄弟のテスト・`lib.rs` に 1 行、案 ⑵ 新しいクレート `crates/real-run-kit/`）・`crates/sample-ghost-kit/Cargo.toml`（案 ⑴ なら `areka-mcp-bridge`・`rmcp` を足す）・`crates/areka/tests/emo2_real_run.rs`・`mcp_get_log_real_run.rs`・`smoke_boot_loop_exit.rs`（移し替え）・`crates/areka/Cargo.toml` の開発の依存（案 ⑵ のとき）・`doc/` の試験の手引きの 1 節。
- 規模: M（8〜12 タスク）。上限 20 の内。
- 先に要るもの: 働きの前提なし。`emo2-real-run-stability` の件 1 とは閉じ方が同じ＝あちらの後に置くか、件 1 の「閉じ方」を本 spec が引き取る（議題 2）。`mcp-kanade-tools` の後だと台本で閉じる道と移し替えの確かめに `sakurascript` が使えて役に立つ（必須ではない）。ファイルの上では `emo2-real-run-stability`（`emo2_real_run.rs`）・`sample-url-fetch`・`default-balloon-selfmade`・`dot-sakura-specimen`（`sample-ghost-kit/src/lib.rs`＝案 ⑴ のときだけ）と同じウェーブに置かない。
- 優先度の区分: **A**（開発者の指示「SSP並みの自動試験が出来るようにすること」「開発がスピードアップすること」）。
- 版: **v0.1.0**。
- 要件定義のモデル: **Opus**。足場の寄せ集めで、MCP の呼び出しは中継が先例、閉じ方は `on_ghost_os_close` が先例。新しい境界・並行の設計・先例で決まらない骨格の分かれ目のどれにも当たらない。
- 要件の段の議題（答えで作業が変わるもの）:
  1. 窓口の置き場: `sample-ghost-kit`（検体の窓口と同じクレート・開発の依存として既に `areka` が引いている）に足すか、新しいクレートにするか。`sample-ghost-kit` は「検体を名前で引く窓口」と自分を説明している＝役が 2 つになる。新しいクレートなら依存の向きがきれい（`real-run-kit` → `sample-ghost-kit`・`areka-mcp-bridge`）。
  2. 閉じ方の持ち主: `emo2-real-run-stability` の件 1（印を見てから OS の閉じる知らせで閉じる）と本 spec のどちらが作るか。推し: 本 spec が窓口の中に作り、`emo2-real-run-stability` の件 1 はそれを使って待ち方だけを直す（同じウェーブに置かず、本 spec が先）。
  3. 二重起動の確かめが無いままでよいか（窓口が立てる areka は番号も根も別なので、並べて立てても取り合わない前提）。`impl-watch-test-desk` が実機の試験を 1 本ずつにするなら、窓口の側の守りは要らない。
