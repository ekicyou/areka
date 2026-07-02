# Implementation Plan

> `areka-parsers` クレートへ `balloon` モジュールを追加し、emo2 バルーン設定を幾何＋フォント subset の型付きモデルへ写像する parser を実装する。model→parse→fixture の一方向依存ゆえ全タスクは逐次（並行機会なし・`(P)` なし）。既存 `sakura`/foundation 規律を全面踏襲（`Result` 無し寛容 facade・NewType/opaque＋read-only accessor・`#[non_exhaustive]`・in-source `#[cfg(test)]`・新規外部依存ゼロ）。

- [x] 1. Foundation: `balloon` モジュールの足場を新設する
- [x] 1.1 `areka-parsers` に `balloon` モジュールを追加し公開面を集約する
  - `crates/areka-parsers/src/lib.rs` に `pub mod balloon;` を 1 行追加する（既存 `charset`/`kv`/`sakura` は無変更）
  - `balloon/mod.rs` を新設し、`mod model; mod parse;` 宣言＋公開 facade/型の `pub use` 集約のみを置く（`sakura/mod.rs`・`kv/mod.rs` 流儀）
  - 新規外部依存を追加しない（`Cargo.toml` 変更なし・std のみ）
  - 観測可能な完了条件: スタブ状態の `balloon` モジュールを含めて `cargo build -p areka-parsers` が成功し、`areka_parsers::balloon` が外部から解決可能
  - `lib.rs` は `shell-parse`/`package-mount` と共有するシームゆえ追加位置のマージ順に留意する
  - _Requirements: 1.1_

- [ ] 2. Core: バルーンモデル型と解析 facade を実装する
- [ ] 2.1 バルーンモデル型を定義し単体テストで契約を固定する
  - `balloon/model.rs` に集約ルート `BalloonModel` と sub-struct（`WindowPosition`/`Origin`/`WordWrapPoint`/`ValidRect`/`Font`/`FontColor`）を定義する
  - 各モデル化スカラを `Option<T>` 直持ちとし、座標成分（x/y・t/b/l/r）と色成分（r/g/b）を個別に `Option` 化して部分欠落を欠落なく表現する（未指定＝`None`）
  - 内部数値型は座標＝`i32`（符号付き）・`font.height`＝`u32`・色成分＝`u8` とし、符号付き座標を保持できる型にする
  - フィールドは非公開とし read-only accessor のみ公開する。全公開 struct に `#[non_exhaustive]` を付す。派生は最小（整数のみの型は `Copy,Eq` 可・`Font` は `String` 含むため `Copy` 不可）
  - `model_tests.rs` を同居させ、公開パス経由で各型を構築し、未指定 accessor が `None` を返して `Some(0)` と区別されることを明示検証する
  - 観測可能な完了条件: `cargo test -p areka-parsers balloon::model` が緑で、未指定値が `None`・指定値が `Some(v)` として accessor から取得でき両者が判別される
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 3.4, 4.1, 4.2, 4.3, 4.5, 5.4, 5.5_
  - _Boundary: balloon::model_

- [ ] 2.2 2 層マージと KV→型写像を担う公開 facade を実装し単体テストで固定する
  - `balloon/parse.rs` に公開 facade `parse(descript, image: Option)`（KV マップ 2 層入口）と便宜入口 `parse_str`（デコード済み文字列を内部で `kv::parse_kv` へ委譲）を実装する
  - 2 層マージ（D4）: descript 基層マップへ画像別層マップを後勝ち `insert` で重ね合わせた 1 マップから 1 回写像する（画像別優先・画像別欠落時 descript 継承・descript のみ許容を単一機構で満たす）
  - マージ済みマップから各モデル化キーを引いて `i32`/`u32`/`u8` へ整数パースし対応スカラへ束ねる。`font.color.{r,g,b}` の 3 キーを個別に引き `FontColor` へ束ねる。負値は符号付きのまま保持しピクセル解決は行わない
  - 寛容: キー不在・非数値・範囲外は当該スカラを `None` へ降格し、未知キー・非モデル化キー（arrow/number/onlinemarker/sstpmarker/sstpmessage/cursor/anchor/communicatebox 等）は無視して継続する。`Result` を返さず panic しない
  - `parse_tests.rs` を同居させ、マージ優先度（画像別優先・descript 継承・descript のみ）・負値保持・非負保持・寛容（未知キー無視・非数値→`None`・空入力→全 `None`）・RGB 部分欠落の個別 `None` を検証する
  - 観測可能な完了条件: `cargo test -p areka-parsers balloon::parse` が緑で、descript のみ入力は descript 値を反映し画像別由来値が全 `None`、画像別層が同一キーを上書きし、`validrect.bottom,-56`→`Some(-56)` 等の符号が保持される
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 3.1, 3.2, 3.3, 3.5, 4.1, 4.2, 4.3, 4.4, 4.5, 2.6, 2.7, 5.4_
  - _Depends: 2.1_
  - _Boundary: balloon::parse_

- [ ] 3. Validation: emo2 実物 fixture で適合を観測する
- [ ] 3.1 emo2-kakukaku fixture 適合テストを追加する
  - `validation_tests.rs` を同居させ、`crates/pilot/examples/shiori-host-32/fixtures/emo2/emo2-kakukaku/` の実物値をリテラル期待値として `parse_str` 経由で検証する
  - R5.1: `descript.txt` 単体 → `origin`(0,0)・`wordwrappoint.x`(-34)・`validrect`(0,0,0,0)・`font.name`(Yu Gothic UI)・`font.height`(28)・`font.color`(0,0,0)、加えて `wordwrappoint.y`=`Some(0)`・`windowposition`=全 `None`
  - R5.2: `descript.txt`＋`balloons0s.txt` → `windowposition`(266,-129)・`wordwrappoint.x`(-49・画像別優先)・`validrect`(46,-56,36,-44)、`origin`/`font` は descript 継承
  - R5.3: `descript.txt`＋`balloonk0s.txt` → `windowposition`(-190,-75)・`validrect`(40,-70,24,-48)、`wordwrappoint.x` は descript の `Some(-34)` 継承、`origin`/`font` は descript 継承
  - 非モデル化キー（arrow/number/onlinemarker/sstpmarker/sstpmessage）が結果へ漏れないこと、および emo2 使用の幾何＋フォント以外の抽象を足していないこと（過剰実装ガード）を確認する
  - 観測可能な完了条件: `cargo test -p areka-parsers balloon` が host 不要・純粋関数のみで緑になり、上記 3 fixture ケースの期待モデルが一致する
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 2.7_
  - _Depends: 2.2_
  - _Boundary: balloon::parse, balloon::model_
