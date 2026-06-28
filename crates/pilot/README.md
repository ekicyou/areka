# crates/pilot — 先進坑の検疫所（quarantine）

`pilot` は二坑モデルにおける**先進坑（pilot）**の使い捨て探索コードと
一次記録を集約する葉ノードクレート（`publish = false`）である。
役割は「検疫所（quarantine）」── 探索的な残骸をこの一葉に閉じ込め、
production クレート（wintf / dola / areka / shiori-abi）を常時クリーンに保つ。

## 空 lib ＝命綱の構造的担保（structural guarantee）

`src/lib.rs` は**意図的に空**で、公開 API（`pub` item）を一切持たない。
探索コードはすべて `examples/` 配下にのみ置く。

- Cargo の `examples/` は他クレートから `[dependencies]` で参照できない。
- 空 lib は依存しても意味のある API を露出しない。

この 2 点により、先進坑コードへの被依存（inbound edge）は
**構造的に発生し得ない**。すなわち「いかなる production クレートも
pilot の探索コードに依存できない」という命綱（葉ノード隔離）が、
機械ガードではなく**クレートの構造そのもの**で担保される。

唯一の inbound 経路は、誰かが他クレートの `Cargo.toml` に
`pilot = { path = ... }` を一行追加することのみであり、これは
レビューで可視な一行変更（かつ空 lib ゆえ実効果なし）として
人手レビューで捕捉する。

## 規律の正本

二坑モデルの規律（先進坑/本坑の定義・命綱・ハードゲート・依存マップ検証・
削除/隔離規律・README 3 幕規約・inbound 依存の人手レビュー規律）の正本は
`.kiro/steering/two-tunnel.md` を参照すること。
