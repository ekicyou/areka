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
