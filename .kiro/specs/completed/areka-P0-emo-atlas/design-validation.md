# 設計バリデーションレポート — areka-P0-emo-atlas

> フェーズ: design-validation（設計確定後・非対話）／言語: ja（spec.json）
> 対象: ⑥ emo トラック直列チェーン 1/3（emo-atlas → emo-compose → emo-present）の素材基盤層
> レビュー基準: `.claude/skills/kiro-validate-design/rules/design-review.md`（Analysis → Critical Issues → Strengths → GO/NO-GO）

## Design Review Summary

本設計は要件 1–6 を Requirements Traceability 表で 1 対 1 に網羅し、steering（クレート責務分割・`XxxResource` 系 CPU 純粋層・unsafe を COM 腕へ集約・Arc 手渡し並行モデル）と記憶（emo 自前合成＋アトラス・parser 転記層・WIC MTA）に整合した、実装準備の整った良質な設計である。ヘキサゴナル軽量版（純粋コア＋デコードポート）により要件 2.3（差替可能・既定手段非露出）と純粋オフスクリーンテスト方針を同時に満たしており、既存 WIC 経路の事実確認（`load_bitmap_source` の存在・PBGRA 化・D2 移設の実現性）もコード実物と一致する。重大な設計不整合はなく、下記の軽微な論点を設計ディスカッションで確認すれば GO。

## Critical Issues (≤3)

### 🔴 Critical Issue 1: `DecodedImage.has_alpha` の取得点が既存 WIC 経路では失われている
**Concern**: 設計は `DecodedImage.has_alpha` を「WIC フレームのピクセルフォーマット（α 有無）から判定」とするが（design.md「デコード層」Implementation Notes）、流用元 `load_bitmap_source`（`crates/wintf/src/ecs/widget/bitmap_source/systems.rs:75`）は **`GUID_WICPixelFormat32bppPBGRA` へ変換した後の `IWICBitmapSource` を返す**（同 93–104 行）。変換後は α 有無情報が消えるため、`load_bitmap_source` をそのまま再利用しても `has_alpha` を導出できない。
**Impact**: D2 の「移設・公開」を字義通り行うと WIC 腕が `has_alpha` を埋められず、契約フィールドが不定になる。emo2 経路（常に α 有・`use_self_alpha=On`）では AlphaChannel 腕に落ちるため実害は出ないが、`has_alpha` はシーム腕（`.pna`/keycolor/full）の分岐入力であり、将来拡張時に静かな誤分岐を招く。
**Suggestion**: WIC 腕は変換前フレームの `GetPixelFormat`（α 付きフォーマットか）を読んで `has_alpha` を確定するよう、移設関数のシグネチャ（または薄い追加ユーティリティ）で pre-conversion フォーマットを露出する旨を design の D2/デコード層 Implementation Notes に一文追記する。挙動不変リファクタの範囲を「返り値の追加（フォーマット情報）」まで含める。
**Traceability**: 2.1, 3.1（α チャンネル採用判定）, 3.3（優先順位分岐）
**Evidence**: design.md「デコード層 › Implementation Notes（`has_alpha` は WIC フレームのピクセルフォーマットから判定）」／「Modified Files（`load_bitmap_source` を `com/wic.rs` へ移設）」

### 🔴 Critical Issue 2: `ManifestDeriver::derive` の返り値 `Vec<AtlasKey>` と重複排除・決定性契約の突合が曖昧
**Concern**: Service Interface は `derive(&self, set) -> Vec<AtlasKey>`（正規化パス昇順・重複なし）と規定するが、間接 bind 参照解決（D6）は「参照先 surface の element を列挙」する一方、下流 packing 入力は `Vec<(AtlasKey, Trimmed)>` であり、`AtlasKey` から実デコード対象パス（`ElementPath` 実体・shell dir 基準の実パス）への対応が導出フェーズと decode フェーズのどちらで解決されるかが明示されていない。`AtlasKey(String)` は「正規化 element パス（無改変保持）」だが、`ElementDecoder::decode(path: &Path)` は「shell dir 基準で解決済みの実パス」を要求する（Preconditions）。両者の橋渡し（相対 element パス→実ファイルパス）が本層内か上流注入かが設計上の空白。
**Impact**: マニフェスト列挙（相対パス保持・1.5）とデコード（実パス要求・2.1）の間に暗黙の解決責務が残ると、実装時に「誰が shell dir を結合するか」で手戻りが生じ、要件 1.5（無改変列挙）と要件 2.1（実デコード）の境界が実装者依存になる。
**Suggestion**: `AtlasKey`（無改変・索引キー）と decode に渡す実パスの対応を、`derive` が `(AtlasKey, PathBuf)` を返すか、または `SurfaceSet` に shell dir を含めて本層が結合する、のいずれかへ設計ディスカッションで確定する（Boundary「element パスは上流注入」との整合も明記）。
**Traceability**: 1.5, 2.1, 6.1
**Evidence**: design.md「列挙層 › Service Interface（`derive -> Vec<AtlasKey>`）」／「デコード層 › Service Interface（`decode(path)` Preconditions: 解決済み実パス）」

### 🔴 Critical Issue 3: `AtlasPage` の割当（頁確保・stride 決定・blit）主体が設計に未記載
**Concern**: `AtlasPage{ width, height, stride, bytes: Arc<[u8]> }` は成果物契約層で「型」として定義され、Packer の Batch 契約は出力を `Vec<AtlasPage>` とするが、**頁バッファの実メモリ確保（`page_size×page_size×4` の割当）・stride 決定・トリム矩形の blit（`Bake` フローの `blit_trimmed`）を担うコンポーネント**が Components 表・File Structure（`pack.rs`/`table.rs`）のどちらに属するか明示がない。`blit_trimmed` は Requirements Traceability 4.3 の Interface 欄に現れるが、所有コンポーネントが Trimmer/Packer に併記され曖昧。
**Impact**: 「packing（座標決定）」と「bake（画素焼付＝頁バッファへの実転写）」は別責務であり、所有が曖昧だと Packer が画素バッファまで抱えて純粋性・単一責務が崩れる、または blit がどこにも属さず実装漏れになるリスク。決定性 golden（画素一致）の検証対象コンポーネントも定まらない。
**Suggestion**: `pack.rs`（座標のみ算出）と頁バッファ焼付（`bake` 入口 or 専用 blit 関数）を明確に分離し、`blit_trimmed` の所有と `AtlasPage` 生成主体を Components 表へ一行追加する。System Flows の `Bake[blit trimmed into page buffers]` ノードに対応する所有者を明記。
**Traceability**: 4.3, 5.1, 6.3
**Evidence**: design.md「System Flows › Bake（`blit trimmed into page buffers`）」／「packing 層 › Batch 契約（出力 `Vec<AtlasPage>`）」／Requirements Traceability「4.3 `blit_trimmed`（Trimmer / Packer）」

## Design Strengths

1. **要件トレーサビリティの完全性と契約の正本化が秀逸**: 要件 1.1–6.5 の全 AC が Requirements Traceability 表で Components/Interfaces/Flows へ 1 対 1 対応し、D3 で `AtlasKey`/`AtlasEntry`/`AtlasPage` を本層が正本定義。空エントリを `placement: Option<Placement>` で型表現し「未知（`get`→`None`）／空（`Some{placement:None}`）／既知」の 3 分岐を型で区別する設計は、下流 emo-compose の再定義を排し Revalidation Triggers も明示していて堅牢。
2. **純粋性とテスト隔離の両立が既存資産の事実に裏打ちされている**: デコードを trait ポートへ隔離し正規化以降を COM 非依存化する方針は、既存 `AlphaMask::from_pbgra32` の stride 込み α 走査・`load_bitmap_source` の PBGRA 化という実証済みパターンの上に立っており（コード実物と一致確認済み）、`rectangle-pack` 承認ゲート・fallback（`rect_packer`）まで含め着手可否が明確。

## Final Assessment

**Decision: GO**

**Rationale**: 重大な設計不整合・要件未充足・過大な複雑度はなく、steering／記憶／既存コードとの整合が取れており実装パスは明快。上記 3 点はいずれも「境界の一文追記・所有の明示」で吸収可能な設計ディスカッション論点であり、emo2 主経路（α 採用・単段 bind）の成立を妨げない。

**Next Steps**:
1. 設計ディスカッション（`/kiro-design-discussion areka-P0-emo-atlas`）で Critical Issues 1–3 を確認・design へ最小追記。
2. 併せて新規依存 `rectangle-pack` の承認を確定（未承認なら着手不可）。
3. 確定後 `/kiro-spec-tasks areka-P0-emo-atlas` でタスク生成へ。
