# Brief: areka-P0-anchor-tag-canon

> 起票: 2026-08-27（bvc 要件ディスカッション議題 5 の開発者指示による `/kiro-discovery` 再入・文字装飾系 3 spec 分割の 2 本目）
> **アンカー（`\_a`・本文中のリンク）機能そのものと、その装飾 16 項目を一括所有する。** `\f` のアンカー系装飾は「アンカー機能ごと不在」のため装飾核 spec から分離した。

## Problem

さくらスクリプトのアンカー `\_a[ID]...\_a`（本文中のクリック可能リンク）が areka に存在しない。`\_a` は `\f` 汎用パススルーと同じ経路で黙って捨てられ、アンカー装飾 16 項目（`anchorstyle`・3 状態の色/フォント/メソッド）は規定する対象が無い。

**⚠実バグを 1 件同梱**: 閉じの素の `\_a` は `lexer.rs:172-177` が `'_'` だけを消費して `Raw("\\_")` を作るため、**後続の `a` が可視のバルーン本文へ漏れる**（2026-08-27 実測・M2 ゲートと独立の生きた欠陥——emo2 は `\_a` を使わないため M1 適合には無害だが、`\_a` を含むゴーストを読み込むと即再現する）。

## Current State

- `\_a[ID]`（開き）・`\_a[ID,引数]`（拡張）・`\_a`（閉じ）の 3 形とも未実装（パススルー→破棄）。`OnAnchorSelect`／`OnAnchorSelectEx` イベントの発火も無い。
- アンカー装飾 16 項目（ukadoc・snapshot 実測）: `anchorstyle`（square|underline|square+underline|none）・`anchorcolor`/`anchorbrushcolor`・`anchorfontcolor`・`anchorpencolor`・`anchormethod`（Win32 `SetROP2` 演算子名）・`anchor.font.color`・`anchornotselect*` ×5・`anchorvisited*` ×5——選択中/非選択/訪問済みの 3 状態。
- descript 側の `anchor.font.*`／`anchor.notselect.font.*`／`anchor.visited.font.*` 族も未解析。`anchor.font.color.r` は現行テストで**拾ってはいけない distractor** として使われている（`parse.rs:157`・`parse_tests.rs:157-162`）＝解析拡張時にこのテストの意図を書き換える。
- **アンカー下線の縦書き写像は bvc が語彙確定済み**——縦書きでは列の**右側**に引かれる（bvc R5.3）。
- 選択肢（`\q`）のクリック機構・hover 描画（`viewbox_draw.rs:346-354` の `SetDrawingEffect`）は既存＝アンカーのクリック/hover の先例。

## Desired Outcome

`\_a` の 3 形が本文中のクリック可能範囲として機能し、`OnAnchorSelect(Ex)` が発火し、装飾 16 項目＋descript `anchor.*.font.*` 族が 3 状態で効き、下線は縦書きで列の右側に出る。素の `\_a` の文字漏れバグが解消している。

## Approach

`text-decoration-canon` の per-run 属性基盤（run 分割・`DWRITE_TEXT_RANGE` 適用）の上に、アンカー範囲＝属性付き run ＋当たり判定（選択肢クリックの機構を先例に）を載せる。lexer の `\_` 2 文字タグ解読の是正（バグ修正）は独立スライスで先行可能。

## Scope

- **In**: `\_a` 3 形の解読・アンカー範囲の保持・クリック/hover・`OnAnchorSelect`/`OnAnchorSelectEx`・装飾 16 項目・descript `anchor(.notselect|.visited).font.*` 族の解析・訪問済み状態の管理・縦書き下線位置（bvc 語彙継承）・**素の `\_a` 文字漏れバグの修正**・決定論テスト。
- **Out**: 装飾の per-run 基盤（`text-decoration-canon`）・選択肢 `\q`/`\__q` の機構（完了済み・不変）・URL 起動等のアンカー既定動作のうち OS 連携部の可否（要件段階で裁定）。

## Boundary Candidates

- lexer バグ修正（`\_` 2 文字タグの正しい消費）は独立・最小・先行可能——**M1 中の前倒し単独修正も選択肢**（開発者裁定次第）。
- アンカー範囲＋イベント（機能）と装飾 16 項目（見た目）の 2 相。

## Out of Boundary

- `\_l` 等ほかの `\_` 系タグの意味論（`cursor-tag-canon` ほか各所有者）——ただし lexer の `\_` 消費規則の是正はタグ名の正しい切り出しとして全 `\_` 系に効く（挙動はパススルー先で不変を保証）。

## Upstream / Downstream

- **Upstream**: `text-decoration-canon`（per-run 基盤・必須先行）・choice-select-events／choice-render（クリック/hover の先例・完了済み）・bvc（下線の縦書き写像）。
- **Downstream**: アンカーを使うゴースト資産の互換（メニュー的トークの主要手段）。

## Existing Spec Touchpoints

- **Extends**: なし（新機能）。
- **Adjacent**: `choice-marker-styling`（同じ 3 spec 群・別レンダラ経路）・`areka-parsers` の distractor テスト（意図の書き換えを伴う）。

## Constraints

- ウェーブ配置: **M2 解禁ゲート**（`text-decoration-canon` の後段）。lexer バグ修正のみ開発者裁定で M1 前倒し可（just-in-time 起票）。
- 決定論テスト必達（3 状態 × 縦横・イベント発火・文字漏れの回帰檻）。
