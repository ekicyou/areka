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

## examples 配置規約（1 仕様 = 1 フォルダ）

先進坑コードは `examples/<spec-name>/` 単位で格納する。**1 仕様 = 1 フォルダ**であり、
各フォルダは次の構成を持つ：

```
crates/pilot/examples/
├── _template/          # 雛形（コピー元・`_` 前置で実 spec と区別・build 対象）
│   ├── main.rs         # 依存ゼロの最小コード
│   └── README.md       # 3 幕 README 雛形（動機→概要→検証結果）
└── <spec-name>/        # 実際の先進坑（1 仕様 = 1 フォルダ）
    ├── main.rs         # 必須。これが無いと Cargo が example として認識しない
    └── README.md       # 一次記録（正本）。3 幕構成
```

- **`main.rs` 必須**: サブフォルダ example は `main.rs` が無いと Cargo に example として
  認識されない（`crates/wintf/examples/taffy_flex_demo/main.rs` で実証済みのサブフォルダ
  example パターン）。フォルダを切ったら必ず `main.rs` を置く。
- **並列安全（merge 衝突ゼロ）**: 各先進坑が独立した自分のフォルダを持つため、複数の先進坑が
  並列に進行しても相互の merge 衝突は発生しない。細粒度・独立ゆえ多重並列で掘れる。
- **実行法**: `cargo run -p pilot --example <spec>` がサブフォルダ example の標準呼出。
  雛形は `cargo run -p pilot --example _template` で動作確認できる。

### 着手手順

1. `_template/` を `<spec-name>/` へコピーする。
2. `main.rs` に探索コードを、`README.md` の 3 幕（動機 / 概要 / 検証結果）を埋める。
3. `cargo run -p pilot --example <spec-name>` で実行・検証する。

> 注: worktree で examples を実際にビルド/実行する際は、submodule（`vendors/pasta`）未populate を
> 避けるため前段で `git submodule update --init --recursive` を要する（既知制約）。

## README 一次記録（3 幕）

各 `examples/<spec-name>/README.md` は当該先進坑の**一次記録（正本）**である。
「動機（なぜ掘るか・対応する本坑 spec の名指し）→ 概要（何を作ったか・実行法）→
検証結果（go/違う/直す ＋ 学び ＋ 日付）」の 3 幕構成で記述する。本坑 spec の design は
この検証結果を参照し、同じ結果を二重化しない。雛形は `_template/README.md` を参照。

## 規律の正本

二坑モデルの規律（先進坑/本坑の定義・命綱・ハードゲート・依存マップ検証・
削除/隔離規律・README 3 幕規約・inbound 依存の人手レビュー規律）の正本は
`.kiro/steering/two-tunnel.md` を参照すること。
