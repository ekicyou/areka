# Brief: areka-P0-mcp-expression-table

> 2026-09-29 `/kiro-discovery` で起票。SSP MCP 移植の **3 段目（個別のツール）**の 1 本。並走の相手と干渉条件は `.kiro/steering/roadmap.md`「SSP MCP の移植」節。事実の正本は [doc/ssp-mcp/survey.md](../../../doc/ssp-mcp/survey.md)。file:line は起票時（main `c3876110`）＝着手時に引き直す。

## Problem

AI エージェントが台本に `\s[n]` を書くには「どの番号がどの表情か」の表が要る。SSP の `get_expression_table` はツール説明で「sakurascript の前にこれを使え」と促しており、AI の台本づくりの起点になる。

## Current State

- シェルの解析は `crates/areka-parsers/src/shell/model.rs`。`aliases: Vec<SurfaceAlias>` は持つが、**descript の見出しは意図して捨てている**（同ファイル 14 行目付近の注記）。表情の説明（名前）の表も `surfacetable.txt` の解析も無い。
- SSP の出力（survey §3）: Markdown 表 `|scope \0,\1,\p[2]...|character name|description|surface number : \s[]|`、行区切り `\r\n`、スコープごとにキャラクタ名と説明と `\s[n]`。**SSP は説明の一部の字を化けさせる**（survey §4-1）＝areka は正しい字で返す。

## Desired Outcome

- `get_expression_table(ghost_name)` が SSP と同じ形の表を返す。
- **説明の出どころ**（`surfacetable.txt`・surfaces.txt の `surface.alias`・`descript` の `sakura.surface.alias` など、どれをどの順で使うか）を要件の段で ukadoc と実物（SSP に同じゴーストを載せた出力）から確定する。
- 表に載る surface の範囲（定義されたもの全部か・説明のあるものだけか）も同じく確定する。

## Approach

パーサに不足する読み取り（`surfacetable.txt` ほか要件で決まるもの）を足し、今のゴーストの今のシェルから表を組む。`mcp-tool-entrances` のダミーの中身を書く。

## Scope

- **In**: 説明の出どころの読み取り（パーサの追加）・表の組み立て・スコープ名（`\0`／`\1`／`\p[n]`）とキャラクタ名の出どころ・決定論テスト（検体のシェルで SSP の出力と突き合わせた期待値）。
- **Out**: 着せ替え・アニメーションの一覧（SSP の本ツールは出さない）。

## Boundary Candidates

- パーサ（`areka-parsers` の shell）と、表の組み立て（MCP ツール）の境。

## Out of Boundary

- シェルの切替・再読み込み（`shell-balloon-switch`・`mcp-reload`）。

## Upstream / Downstream

- **Upstream**: `mcp-tool-entrances`。
- **Downstream**: なし（`mcp-dump-images` の「No such surface ID. Check get_expression_table tool」は文言で指すだけ）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `shell-balloon-switch`（α・シェルの載せ替え。同じ `shell/` を読む）。

## Constraints

- 規模 S〜M。SSP の出力との突き合わせは開発者の机の SSP（`C:\wintools\ssp`）で要件の段に 1 度とる（常時テストへは期待値の文字列として固定する）。


---

## 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- 棚卸⑳では個別の再測定をしていない（`mcp-tool-entrances` が、各 spec の触るファイルを設計で固定する）。着手は `mcp-tool-entrances` の完了の後で、そのとき接触ファイルを照合する。

## 2026-10-03 ウェーブ C3-⑦（予定・10-03 の再編（開発者「MCP は複合 spec なので早めに着手したい」））

- 段は「優先」。`mcp-tool-entrances`（C2）の design が固定した「自分のツールのファイル」と、同じ C3 の他の spec（`balloon-lifecycle-events` は kanade・`balloon-font-file` は emo-text）の触るファイルを、着手の前に照合する。

## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 規模: S〜M（6〜10 タスク）。切らない。
- 前提の状態: `mcp-tool-entrances` は着地済み（PR#223）。前提は満たす。
- 崩れた前提／古くなった位置:
  - ダミーの場所: アプリ本体側 `crates/areka/src/mcp/get_expression_table.rs` の `handle`（`NG:not implemented yet` の 1 文）と `get_expression_table_tests.rs`（同じ文言を期待＝書き換える）。プロトコル側 `crates/areka-mcp/src/tools/get_expression_table.rs` は定義と `Args { ghost_name: Option<String> }` まで完成＝触らない見込み。`ghost_name` の省略・空は `dispatch` が `Omitted::Reject` で `NG:Specified ghost is not active` にしてから来る（ここで扱わない）。
  - 今のシェルの所在: `GhostSession::current_shell_folder()` と `ghost_dir()`（`crates/areka/src/ghost_session.rs`）。キャラクタ名は `GhostSession::names()`（`GhostNames`）。
  - `areka-parsers` の shell は今も descript の見出しを捨て（`shell/model.rs` の冒頭の注記「descript ヘッダ・charset はモデルに保持しない」）、`surfacetable.txt` の解析は無い（ソース全域で 0 件）。`Shell.aliases`（`SurfaceAlias`・surfaces.txt の `sakura.surface.alias` 等のブレス）はある。
  - `shell-balloon`（C2）が `shell/model.rs`・`decode.rs` へ `balloon.名前`ブレスの読み取りを足した。同じ 2 ファイルは C3 の `surface-element-nesting` も触る（「シェルの element」の列）。
- 触るファイル（並走の照合用）:
  - `crates/areka/src/mcp/get_expression_table.rs`・`get_expression_table_tests.rs`
  - `crates/areka-parsers/src/shell/` に**新規**の読み取り（例 `surfacetable.rs`＋兄弟テスト）と `shell/mod.rs` の `mod`／`pub use` の行。**`model.rs`・`decode.rs` の `Shell` へ欄を足さない形を先に探す**（`surfacetable.txt` は別ファイルなので別の型で返せる）。足すなら `surface-element-nesting` と同時に走らせない。
  - 触らない: `crates/areka/src/mcp/mod.rs`・`resolve.rs`・`crates/areka-mcp/src/**`・`Cargo.toml`
- 議題（答えで作業が変わるものだけ）:
  - 説明の出どころと順（`surfacetable.txt`／surfaces.txt の alias／descript）と、表に載せる surface の範囲（定義全部か説明のあるものだけか）——SSP の実測（開発者の机の SSP）で 1 度とる。答えで「パーサに何を足すか」が変わる。
- 見つけた穴: なし。

## 2026-10-04 ウェーブ C3-⑧（棚卸㉑）

- 段は「優先」。C3 は 11 本並走（`roadmap.md`「ウェーブ編成」の C3 の行が正本）。着手は最新の main から。
- 同じウェーブの約束: `crates/areka/src/mcp/mod.rs`・`handler.rs` を触らない。表の読み手は新規 `crates/areka-parsers/src/shell/surfacetable.rs`＋`shell/mod.rs` の 1 行で、`Shell` 型（`model.rs`・`decode.rs`＝C3-⑤ の場所）に欄を足さず別の型で返す。
