# Brief: areka-P0-mcp-log-history

> 2026-09-29 `/kiro-discovery` で起票。SSP MCP 移植の **3 段目（個別のツール）**の 1 本。並走の相手と干渉条件は `.kiro/steering/roadmap.md`「SSP MCP の移植」節。事実の正本は [doc/ssp-mcp/survey.md](../../../doc/ssp-mcp/survey.md)。file:line は起票時（main `c3876110`）＝着手時に引き直す。

## Problem

AI エージェントは台本を流した後、「エラーが出たか」「何が再生されたか」をログで確かめる（SSP の `get_log`・`since_id` で差分だけ読む）。areka のログは標準エラーへ流れて消えるだけで、動いているアプリへ問い合わせて読めない。

## Current State

- ログは `tracing_subscriber::fmt()` だけ（`crates/areka/src/main.rs` 143〜148 行目付近）。独自の Layer・メモリ上の履歴・通し番号は無い。
- `log-capture-kit`（テスト専用・`publish = false`）の捕捉 Subscriber は形の手本になるが、アプリから依存してはならない。
- SSP の形（survey §3）: 種別 5 つ（error 既定／script／network／update／status）、1 行 `#<id> <yyyy/mm/dd hh:mm> [<種別>] <名> : <本文>`、`\r\n` 区切り、古い順、継続行はタブ始まり、0 件は `(no log entries)`、**id は全種別で通し**、`since_id`・`max_count`（新しい方から）・`ghost_name` で絞る。**SSP は script の本文の逆斜線を JSON でエスケープし損ねる**（survey §4-2）＝areka は正しく返す。

## Desired Outcome

- アプリの `tracing` の出来事のうち、5 種別に当たるものが通し番号付きでメモリに残り（上限あり・古いものから捨てる）、`get_log` で SSP と同じ書式で読める。
- **種別への振り分けの規則**（error＝warn 以上＋strict の記録、script＝再生した台本、network／update＝`areka-update`・インストールの出来事、status＝起動・読み込みの節目）を要件で決め、tracing の **target 名の取り決め**として文書にする（出す側の spec はこの名前で出す）。

## Approach

`tracing_subscriber` の Layer を 1 つ足し、取り決めた target と level で種別を決めて環状の履歴へ積む。履歴は `mcp-strict-errors` が error 種別へ書き込む口も持つ（直接 `tracing` で出してもよい＝design で決める）。

## Scope

- **In**: 履歴の Layer と上限・種別の規則と target 名の取り決め・`get_log` の中身・network／update／status の出す側（`areka-update`・インストール・起動の既存の行に target を付ける）・決定論テスト。
- **Out**: script 種別を出す 1 行（kanade 側＝`mcp-kanade-tools`）・strict の記録（`mcp-strict-errors`）・ログのファイル保存・ログ窓の UI。

## Boundary Candidates

- 「何を残すか」（target と level の規則）と「どう読むか」（書式・絞り込み）の境。
- 出す側（各エンジン）と履歴の間は tracing の target 名だけで結ぶ（コードの依存を作らない）。

## Out of Boundary

- 既存の `fmt` 出力の書式を変えること。
- SSP の開発者ツールのログ窓のような表示。

## Upstream / Downstream

- **Upstream**: `mcp-tool-entrances`。
- **Downstream**: `mcp-strict-errors`（error 種別へ記録）。`mcp-kanade-tools` は取り決めた target で script の行を出す（並走・コードの依存なし）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `network-update`（α・`areka-update` の行を触る。α の着地後に本 spec が target を足す）。

## Constraints

- ログ檻の盲点に注意: bevy_ecs は `log::` で出すので tracing の捕捉に届かない（記憶 areka-log-cage-harness-blindspots）。
- 1 行の長さ・履歴の上限でメモリを食い潰さない（上限の数は要件で決める）。


---

## 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- 棚卸⑳では個別の再測定をしていない（`mcp-tool-entrances` が、各 spec の触るファイルを設計で固定する）。着手は `mcp-tool-entrances` の完了の後で、そのとき接触ファイルを照合する。

## 2026-10-03 ウェーブ C3-⑧（予定・10-03 の再編（開発者「MCP は複合 spec なので早めに着手したい」））

- 段は「優先」。`mcp-tool-entrances`（C2）の design が固定した「自分のツールのファイル」と、同じ C3 の他の spec（`balloon-lifecycle-events` は kanade・`balloon-font-file` は emo-text）の触るファイルを、着手の前に照合する。

## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 規模: M（8〜12 タスク）。切らない。
- 前提の状態: `mcp-tool-entrances` は着地済み（PR#223）。前提は満たす。`network-update` も完了済み（Adjacent の「α の着地後」は済んだ）。
- 崩れた前提／古くなった位置:
  - ダミーの場所: アプリ本体側 `crates/areka/src/mcp/get_log.rs` の `handle(_world, _args, reply)`（`ghost_name` を解決しない形・`dispatch` の `ToolCall::GetLog` の腕）と `get_log_tests.rs`（`NG:not implemented yet` を期待＝書き換える）。プロトコル側 `crates/areka-mcp/src/tools/get_log.rs` は `Args { log_type, ghost_name, since_id: Option<i64>, max_count: Option<i64> }` まで完成。`ghost_name` の空の文字列はそのまま届く（tool-entrances の design「`get_log` には空のまま届く」）＝空の扱いは本 spec が決める。
  - tracing の初期化は今も `crates/areka/src/main.rs` の `fn main()` の先頭の `tracing_subscriber::fmt().with_env_filter(…).init()` の 1 か所。`main.rs` は 957 行（上限 1,000）。tool-entrances の約束では `main.rs` は「3 段目が触らない共有ファイル」＝本 spec は理由を要件に書いて数行だけ触る（`registry().with(fmt 層).with(履歴の層)` への組み替え）。
  - 依存は足さずに済む: `tracing-subscriber`（ワークスペースの `version = "0.3", features = ["env-filter"]`・既定機能に `registry`・`fmt`・`tracing-log` を含む）で層を重ねられる。時刻の `yyyy/mm/dd hh:mm`（現地時刻）は `windows` の `Win32_System_SystemInformation`（宣言済み）の `GetLocalTime` で足りる。
  - 層を差す書き方の注意: `log-capture-kit/tests/with_default_guard_test.rs` が「捕捉先を直接差す呼出」の新設をワークスペース全体で見張っている。`SubscriberInitExt::init()` で組めば当たらない見込みだが、着手時に見張りの 3 語と照らす。
  - 出す側の行に target を足す案は触るファイルが多い（`crates/areka-update/src/`・`crates/areka/src/update/`・`crates/areka/src/install/`・起動の節目＝`ghost_session.rs`・`emo2_boot/ghost_switch.rs`）。いずれも他の spec（`install-companion-reading`・`install-live-target-hazards`・`network-update-canon-order`・`mcp-reload`）と重なる。tracing の既定の target はモジュールのパス（`areka_update::…`・`areka::install::…`）なので、**既存の target とモジュールのパスで振り分ける**なら出す側を 1 行も触らずに済む。
- 触るファイル（並走の照合用）:
  - `crates/areka/src/mcp/get_log.rs`・`get_log_tests.rs`
  - **新規**の履歴の層（例 `crates/areka/src/log_history.rs`＋兄弟テスト。`mcp/` の下に置くなら `mcp/mod.rs` を触らないよう `get_log.rs` の子モジュールにする）
  - `crates/areka/src/main.rs`（初期化の組み替えと `mod` 1 行）
  - target の取り決めの文書（`doc/` の下・新規。`mcp-kanade-tools`・`mcp-strict-errors` が読む）
  - 出す側の行は上の振り分けを選べば 0 件
- 議題（答えで作業が変わるものだけ）:
  - 種別への振り分けを「出す側の行に target を足す」か「既存のモジュールのパスと target で振り分ける」か（前者は 5 か所以上のファイルを触り並走を崩す・後者は 0 件）。
  - 履歴の層の絞り込みは `RUST_LOG` と独立にするか（全体の `EnvFilter` の下に置くと、利用者が `RUST_LOG=areka_mcp=debug` のように絞ったとき error 種別が履歴に残らない＝層ごとの filter にする必要がある）。
- 見つけた穴: なし。

## 2026-10-04 ウェーブ C3-⑨（棚卸㉑）

- 段は「優先」。C3 は 11 本並走（`roadmap.md`「ウェーブ編成」の C3 の行が正本）。着手は最新の main から。
- 同じウェーブの約束: `crates/areka/src/mcp/mod.rs`・`handler.rs` を触らない。出す側の行に target を足さず、既存のモジュールのパスで振り分ける（`install/`・`update/`・`areka-update` に触らない）。`main.rs` は tracing の初期化だけ（理由を要件に書く）。
