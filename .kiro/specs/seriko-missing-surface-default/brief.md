# Brief: seriko-missing-surface-default

## Problem
台本の `\s[n]` が、今のシェルに無い面の番号を指したときの扱いが ukadoc と食い違っている。ukadoc は `seriko.defaultsurface` を「存在しない番号を\sタグで指定した時などに代わりに表示する」面と書いている。一方、今の areka は無い面の `\s` を記録に残すだけで、表示を変えない。ゴーストの作者が面の番号を書き違えたとき、ukadoc を読んで作った人が期待する「既定の面に戻る」が起きない。

## Current State
- `descript-window-keys`（2026-10 完了）が、ゴーストの descript からスコープごとの既定の面を読む部品 `placement::DefaultSurfaces`（候補の並び＝descript の面、違えば組み込みの面）を入れた。これは窓の大きさの測り・起動の最初の面・読み直しとシェルの切り替えの戻り先にだけ使っている。
- 台本の `\s` が無い面を指したときは、seriko が `surface not found` の類の記録を残し、今の面のまま（表示を変えない）。
- SSP 2.9 の測り（2026-10-23・ぱすた・既定の面 1000）: `\0\s[9998]\e`（strict）を送ると、誤りの記録に `surface not found (9998)` が 1 件、`currentghost.scope(0).surface.num` は `-1`、既定の面 1000 は出ず、rect も変わらなかった。SSP は ukadoc の字面どおりには動いていない（根拠: `.kiro/specs/completed/descript-window-keys/requirements.md` の裁定 11・`research.md` の SSP の測りの節）。

## Desired Outcome
- 台本の `\s[n]` が今のシェルに無い面を指したときの扱いが、決めた方針（ukadoc の字面＝既定の面を出す、または SSP の実測＝面を出さず番号を -1 にする）どおりになり、決定論のテストで固定されている。
- どちらを採ったかと根拠（ukadoc の引用・SSP の測り）が互換の対応表 §8 と網羅台帳に載っている。

## Approach
要件の段で、まず ukadoc の字面と SSP 2.9 の実測のどちらに合わせるかを開発者に決めてもらう（areka の方針は「意味論は ukadoc から輸入・ukadoc で読めない点は SSP で測る」。今回は ukadoc が書いてあり SSP が違う＝食い違いの扱いそのものが議題）。推奨は ukadoc の字面（既定の面を出す）で、既定の面は `placement::DefaultSurfaces` の候補の並びの最初に在る面を使う（読み直しの戻り先と同じ決め方）。

## Scope
- **In**: 台本の `\s[n]`（`\0`・`\1`・`\p[n]` のスコープ）が無い面を指したときの表示の扱い・記録・決定論のテスト・網羅台帳と §8 の行。
- **Out**: 窓の大きさの測りと起動の最初の面と読み直し／シェルの切り替えの戻り先（`descript-window-keys` で済み）。`\s[-1]`（面を消す）の扱い。面の別名（`alias.txt`・`ghost-alias-file` の持ち分）。

## Boundary Candidates
- seriko の面の切り替え（`\s` を受けて今の面を決める所）
- 既定の面の値をどこから seriko へ渡すか（シェルの差し替えで渡している戻り先の材料と同じ道を使えるか）

## Out of Boundary
- 面の合成とアニメーションそのもの
- 面の別名の引き当て（`ghost-alias-file`）

## Upstream / Downstream
- **Upstream**: `descript-window-keys`（`placement::DefaultSurfaces`・seriko の `ShellSurfaceFallback`）
- **Downstream**: なし（ukadoc の拾い残しの 1 件）

## Existing Spec Touchpoints
- **Extends**: なし（新しい境界）
- **Adjacent**: `ghost-alias-file`（面の引き当ての順）、`seriko-pattern-calls`（seriko の面の命令）

## Constraints
- 区分 C（ukadoc の拾い残し・バグでない持ち越し）・段は「その他」・版は「いつか」。
- `descript-window-keys` の裁定 11 による起票（同 spec の完了の記録に起票したことを残す）。
- 要件の段で SSP を測り直すなら、毎回開発者に許しを取る。
