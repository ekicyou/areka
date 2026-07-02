# 設計バリデーションレポート — areka-P0-balloon-parse

> 種別: design-validation（kiro-validate-design 成果・非対話実行）。入力: `spec.json`(language=ja)／`requirements.md`(確定)／`design.md`(確定)／`research.md`／steering・ukadoc。プロセス: Analysis → Critical Issues → Strengths → GO/NO-GO。design.md/requirements.md/research.md/spec.json は変更していない。

## 設計レビュー要約

本設計は emo2 バルーン設定を「幾何＋フォント subset の型付きモデル」へ写像する parser を、確立済みパーサー規律（`sakura`/foundation）に全面準拠して定義しており、要件 R1〜R5 の全 28 受入基準がトレーサビリティ表で被覆され、境界・ファイル構成・型契約が具体値で埋まった実装可能な設計である。核心契約（None≠0、emo2 subset 限定、寛容 facade、new 外部依存ゼロ）はいずれも型と設計判断（D1–D9）で保証されている。品質は実装移行に十分で、致命的な整合破綻は検出されなかった。

## 重点確認結果（本 spec 契約に対する検証）

### 1. None を 0 と区別する契約（R2.6/R3.4・討論 #1）— 満たす
- 各モデル化スカラを `Option<T>` 直持ちとし、座標成分（x/y、t/b/l/r）・色成分（r/g/b）を**個別に** `Option` 化（design「Data Models」D3、公開型定義）。accessor は `Option<T>` を返し「未指定」を明示伝達。
- 「権威なき既定値・ゼロ値で代替しない」を Overview Goals・Boundary・Error Handling（入力欠落＝正常系 `None`）で一貫して明文化。built-in default 焼き込みの経路は設計上存在しない。
- `model_tests` で「未指定 accessor が `None` を返し `Some(0)` と区別される」ことを明示検証（Testing Strategy）。契約が型かつテストで二重に固定されている。

### 2. emo2-only スコープ規律 — 満たす
- choice/link/scroll 系キー（cursor/anchor/number/arrow/sstpmarker/sstpmessage/onlinemarker/communicatebox/marker）を Non-Goals・Out of Boundary・R2.7 除外・Error Handling（無視）で明確に非モデル化。
- 拡張シームは `#[non_exhaustive]` のみに限定（投機的抽象なし）。内部分割も D8 で `model.rs`＋`parse.rs` の 2 本に flatten し `sakura` の 4 層分割を YAGNI として回避。過剰実装ガード（R5.5「2 例目まで抽象を足さない」）を Testing Strategy に明記。

### 3. パーサー規律の忠実性 — 満たす
- 公開 facade `parse(...) -> BalloonModel` / `parse_str(...) -> BalloonModel`（`Result` 無し・寛容）で R1.1/R1.2。異常は全て「該当スカラ `None` 降格」で吸収し panic しない。
- フィールド非公開＋read-only accessor（NewType/opaque 流儀）で R2.8。全公開 struct に `#[non_exhaustive]`。
- in-source `#[cfg(test)]`（model/parse/validation 3 分割）で host 不要・純粋（R5.4）。
- 新規外部依存ゼロ（std のみ：`BTreeMap`・`str::parse`）。`tracing` は既存依存・任意。Boundary で「新規外部依存の追加は禁止」を明記。

### 4. 要件トレーサビリティ — 満たす
- R1.1〜R5.5 の全 IDが Requirements Traceability 表に Component/Interface 対応付きで存在（orphan なし）。
- emo2 fixture マージ期待（R5.1〜R5.3）は `validation_tests` で `parse_str` 経由の具体期待値（windowposition 266/-129・-190/-75、wordwrappoint.x -34/-49 継承、validrect 各値、font subset）として単体テストで検証可能。research §1.4 実測表と R5 期待値の一致も確認済。

## 致命的イシュー

なし（0 件）。3 重点契約すべてが型・設計判断・テストで担保され、致命的な整合破綻・要件ギャップは検出されなかった。

以下は致命的ではない軽微な留意点（設計討論の任意論点・GO を妨げない）:

- **[軽微・非致命] `lib.rs` 共有シームのマージ順**: `pub mod balloon;` 追加行は `shell-parse`/`package-mount` と共有シーム。Revalidation Triggers・Risks で既に注記済で、並列 spec とのマージ競合は順序留意で回避可。設計対応不要。
- **[軽微・非致命] `parse_str` の入力前提**: `parse_str` の Precondition が「UTF-8/デコード済み」＝charset は上流責務。これは Boundary（charset 非所有）と整合しており正しいが、呼び出し側が生バイトを渡す誤用余地は API doc コメントで注意喚起すると親切（実装時の doc 事項・設計変更不要）。

## 設計の強み

1. **契約担保の二重化**: None≠0 という要件討論 #1 の核心制約を、型（成分別 `Option<T>` 直持ち）と in-source テスト（`Some(0)` との区別を明示検証）の両面で固定しており、下流ピクセル解決の前提が崩れない設計になっている。
2. **既存規律の全面踏襲＋YAGNI 徹底**: `sakura`/foundation の facade・NewType/opaque・`#[non_exhaustive]`・in-source テスト規律を一貫適用しつつ、ロジックの軽量さに合わせて分割を 2 ファイルに flatten し、正典 ukadoc に符号意味を委ねて再定義を避けている。非侵襲な接ぎ木（`lib.rs` 1 行）で並走安全。

## 最終評価

- **判定: GO**
- **根拠**: 全 28 受入基準が被覆され、None≠0・emo2 subset 限定・寛容 facade・新規依存ゼロの 4 核心契約が型と設計判断と単体テストで保証されている。技術的未知はなく（std のみ・fixture/正典 確定済）、致命的な整合破綻もない。実装移行に十分な品質。
- **次段階**: `/kiro-spec-tasks areka-P0-balloon-parse` でタスク生成へ進む。軽微留意点（`lib.rs` マージ順・`parse_str` doc 注記）は実装時に吸収可能で、設計改訂は不要。
