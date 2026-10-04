# Brief: areka-P0-mcp-ghost-name-match

> 2026-10-04 `/kiro-discovery` で起票（`mcp-get-property` の要件ディスカッション 議題 2 で開発者が「起票する」と裁定）。事実の正本は [doc/ssp-mcp/survey.md](../../../doc/ssp-mcp/survey.md) §7.2・§7.4（SSP 2.9.07 の実測）。MCP は ukadoc に索引されていない＝survey（SSP の実測）が MCP の約束の正本。コードは「何の定義か」で指す。着手時に引き直す。

## Problem

AI エージェントが MCP のツールに渡す `ghost_name` の照合が、SSP と 4 点ずれている。SSP で通る指定が areka では `NG:Cannot find active ghost from specified name` になり（大小違い・本体側名・前後の空白）、SSP で外れる空文字が areka では起動中の 1 体に当たる。全 10 ツールに共通する。

## Current State

- 照合は `crates/areka/src/mcp/resolve.rs` の `resolve`（純粋な判断）1 か所。起動中のゴーストの名前とルートフォルダは同じファイルの `active` が World から組む（`ActiveGhost { name, root }`・本体側名は持たない）。本体側名は `GhostSession::names()`（`crates/areka/src/ghost_session.rs`）が返すマウントの名前にある見込み（着手時に確かめる）。
- 今の形は完了 spec `mcp-tool-entrances` の要件 3.2〜3.5 と暫定の裁定 6・7（「descript の `name` と完全一致（大文字小文字も区別）」「`sakura.name` などの別名では照合しない」「空の文字列は省略と同じ」）。その裁定は SSP で確かめる前に決めたもの。テスト `crates/areka/src/mcp/resolve_tests.rs` が今の形を固定している（`name_differing_only_in_case_does_not_resolve`・`sakura_name_does_not_resolve`・`empty_or_omitted_with_use_active_resolves_to_the_active_one`・`empty_or_omitted_with_reject_is_not_active` など）。
- SSP 2.9.07 の実測（survey §7.4）:
  - 名前の英字の大小を区別しない（`えも2debug` が通る）。
  - 本体側名（`sakura.name`＝`むらさき`）で通る。相方の名前（`kero.name`）・かなの違いは通らない。
  - 名前の前後の空白は無視して通る。パスの前後に空白があると通らない。
  - 空文字・空白だけは `NG:Cannot find active ghost from specified name`（`get_expression_table` でも同じ。areka は `get_expression_table` の空文字に `NG:Specified ghost is not active` と答える）。
  - パスの照合（大小・`\` と `/`・末尾の区切りを同じとみなす・フォルダ名だけは不一致）は areka と同じ。

## Desired Outcome

- `ghost_name` の照合が survey §7.4 の表と同じ答えになる（4 点のずれが無くなる）。
- 省略（引数が無い）と空文字が区別される。
- 今の形を固定していたテストが、SSP の実測の形の期待へ書き換わっている。

## Approach

`resolve` の判断を survey §7.4 に合わせて直し、`active` に本体側名を足す。宛先の解決は全ツールが通る 1 か所なので、各ツールのファイルは変えない。照合の細部（英字の大小の畳み方＝ASCII だけか・空白の削り方＝名前だけでパスは削らない・本体側名と名前が別のゴーストで重なったときの順）は要件の段で決める。

## Scope

- **In**: `resolve.rs` の照合の判断と `active` の本体側名・`resolve_tests.rs` の書き換え・`get_expression_table` の空文字の答えの文言（SSP は `Cannot find…`）・決定論テスト・実機確認（SSP と並べて同じ指定を当てる）。
- **Out**: 起動中の一覧（`get_active_ghost_list`）の値の形・プロパティの名前の大小（`property-name-case-fold`）・多重起動（2 体以上）の照合の順。

## Boundary Candidates

- 宛先の解決（`crates/areka/src/mcp/resolve.rs`・`resolve_tests.rs`）。
- 必須の `ghost_name` を省略・空で拒む所（`get_expression_table` の `Omitted::Reject`・`crates/areka/src/mcp/mod.rs` の `dispatch` から渡る）。

## Out of Boundary

- 各ツールの処理（`crates/areka/src/mcp/<ツール>.rs`）。宛先が解決された後のことは変えない。
- プロトコル側（`crates/areka-mcp/`）の引数の検査。

## Upstream / Downstream

- **Upstream**: `mcp-tool-entrances`（完了・今の照合の持ち主）。
- **Downstream**: MCP の 3 段目の全ツール（`ghost_name` を受けるもの）。`mcp-get-property` は宛先の解決に頼るだけで、照合をテストで固定しない（同 spec の要件で約束・完了のときに本 brief へ申し送りを書く）。

## Existing Spec Touchpoints

- **Extends**: なし（`mcp-tool-entrances` は完了＝消化できない。その要件 3.2〜3.5 と裁定 6・7 を本 spec が上書きする）。
- **Adjacent**: MCP の 3 段目の各 spec（`mcp-get-property`・`mcp-expression-table`・`mcp-log-history`・`mcp-dump-images`・`mcp-kanade-tools`・`mcp-reload`・`mcp-strict-errors`）。

## Constraints

- 段は「優先」（SSP MCP の移植）。規模の見立て S（3〜5）。
- **共有ファイルを触る**: MCP の 3 段目の約束は「`mcp/mod.rs`・`handler.rs` を触らない」。`resolve.rs`・`resolve_tests.rs` も全ツールの共有の土台なので、**3 段目の spec と同じウェーブに置かない**。席は 3 段目が着地した後（`mcp-strict-errors` の後）か、3 段目が走っていないウェーブ。`mcp/mod.rs` を触らずに済むかは要件の段で確かめる（`get_expression_table` の空文字の文言は `resolve` の中で決まる見込み）。
