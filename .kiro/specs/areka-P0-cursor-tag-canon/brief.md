# Brief: areka-P0-cursor-tag-canon

> 起票: 2026-08-27（`areka-P0-balloon-vertical-canon`〔bvc〕要件ディスカッション議題 3 の開発者裁定による即日起票・`/kiro-discovery` 再入 Path C）
> **開発者裁定（起票の根拠）**: 「`\_l[x,y]` はやるなら同じタイミングで全部実装しないといけない。全仕様を対象とする spec を作り、bvc では一切実装しない」——つまみ食い実装（負値だけ・縦書きだけ等の部分解禁）の**禁止**が本 spec の存在理由である。

## Problem

さくらスクリプトのカーソル移動タグ `\_l[x,y]` は、ukadoc が定める全語彙のうち**一部だけ**が areka に実装されている。SSP 2.8.83（2026-08-26）が縦書きバルーンの `\_l` 座標系を明文化したことで、未実装部分（負値・`@` 相対・`%`）が「後回しでよい飾り」から「縦書きの正規の列指定手段を含む欠落」へ変わった——縦書きでは**負の絶対 X が 2 列目以降を指す正規の書き方**である（原点が文字描画範囲の右上へ移るため）。

部分的に門を開ける（例: 縦書きの負値だけ解禁）と、`%` や `@` を使う SSP 向けスクリプトが依然転び、しかも「対応済み」の顔をするため発見が遅れる。カーソル座標の解決は単一の縮退表で守られており、語彙ごとに別々の時期に触ると縮退表の一貫性が壊れる。

## Current State

2026-08-27 の bvc ギャップ分析（`areka-P0-balloon-vertical-canon/research.md` §3.5）による実測。file:line は当日検証値。

| ukadoc 語彙 | areka 現状 |
|---|---|
| 数値（文字描画範囲**左上**からの px 絶対座標） | ✅ 実装済み・ただし**非負のみ**（`layout.rs:656-670` の `value >= 0.0` ゲート） |
| （省略）＝当該軸は動かさない・両省略で無効果 | ✅ 実装済み |
| `XXem`（文字高さ基準・小数可） | ✅ 非負のみ |
| `XXlh`（行高さ＝1em＋行間・小数可） | ✅ 非負のみ |
| `XX%`（文字高さ基準・100%＝文字高さ・小数可） | ❌ 未実装 |
| `@XX`（現在位置からの相対・負＝左/上・em/% と共存可） | ❌ 未実装（`CursorCoord::Relative` は `layout.rs:684` の `CursorDegrade::Relative` へ縮退） |
| 負値絶対座標 | ❌ 拒否縮退（`CursorDegrade::NegativeAbsolute`・`layout.rs:679`・当該軸不動＋actor ごと warn-once） |
| 副作用（`\_l` 直後の行揃え左寄せリセット・`\f[align]` とのインデント相互作用・`\c[line]` の「行」分割単位） | ❌（`\f[align]` 系自体が全書字方向で未実装＝M2 予約） |

**縮退表の正典は完了 spec `areka-P0-emo-text-layer`**（R2.4／6.5 が 4 分岐縮退——NegativeAbsolute／Relative／Invalid／Omitted——を確定し `CursorWarnGuard` が檻にしている）。本 spec はこの縮退表を改訂する＝**完了 spec の正典改訂**であり、裁定時は design・境界節・steering への追随義務を負う（記憶: revise-design-not-just-requirements）。

**縦書き `vertical_rl` は原点と符号が正典と食い違う（bvc 実測・本物の非互換）**:
- カーソル X は常に `region.left()` 起点の増加方向で解決される（`layout.rs:453-454`）。
- 一方 `vertical_rl` の列送りは `validrect.right` 起点の減少方向（軸読み替え正準表 `layout.rs:305-311`＝`(start.1, start.0, -1.0)`）で、列矩形は `[block_pos - font_height, block_pos]`＝`block_pos` は列の**右端**（`finish_line`・`layout.rs:611-621`）。
- 帰結: `\_l[0,0]` が 1 列目ではなく**文字描画範囲の外側左方**に着地する（フィクスチャ値 validrect 36..356・font 28px で x∈[8,36] に列が立つ・自然な 1 列目は x=356）。
- **`vertical_lr`（areka 拡張）は既に整合**（`(start.1, start.0, +1.0)`・列矩形 `[block_pos, block_pos + font_height]`＝`\_l[0,0]` が自然な 1 列目と厳密一致）。

**テスト被覆**: `layout_cursor_tests.rs`（670 行・13 本）は `WritingMode::` 全 22 箇所が `HorizontalTb`＝**縦書き `\_l` の被覆 0**。この空白が上記欠陥の温床だった。

## Desired Outcome

`\_l[x,y]` の ukadoc 全語彙（数値／省略／em／lh／%／@ 相対／負値）が、3 書字方向（horizontal_tb／vertical_rl／vertical_lr）すべてで正典どおりに解決され、決定論テストで固定されている。縦書き `vertical_rl` では `\_l[0,0]`＝1 列目の先頭・負 X＝次の列。副作用のうち実装可能なもの（`\c[line]` の行分割単位等）は実装し、未実装機能に依存するもの（`\f[align]` リセット・インデント相互作用）は写像を語彙登記して当該機能の追跡先へ申し送る。

## Approach

**一括実装（アトミック）**。語彙単位の分割着地は開発者裁定で禁止。実装順の内訳（設計フェーズで確定）はおよそ:
1. 縮退表の改訂（完了 spec `emo-text-layer` の正典改訂＋追随）——負値解禁・`Relative` 実装化・`%` 追加で 4 分岐が縮む。
2. 座標解決の書字方向対応——`vertical_rl` の列送り軸原点・符号の是正（`\_l[0,0]`＝1 列目）。`draw.rs` 側の再レイアウト（`rect.left`／縦書き `rect.top` 起点の組み直し・`layout.rs:601-610` 結合コメント）と可視窓あふれ判定（`layout_visible_window_tests.rs`）との 3 者整合は bvc research §7 R-1 を参照。
3. `%`・`@` 相対（em/% との共存込み）の新規実装。
4. 3 書字方向 × 全語彙の決定論テスト網（現在縦書き被覆 0 からの新設）。

## Scope

- **In**:
  - `\_l[x,y]` の全座標語彙: 数値（負値含む）／省略／`em`／`lh`／`%`／`@` 相対（em/% 共存込み）。
  - 3 書字方向すべてでの座標系正典化（horizontal_tb＝従来どおり・`vertical_rl`＝SSP 2.8.83 正典・`vertical_lr`＝areka 拡張として `vertical_rl` の鏡像を維持）。
  - 完了 spec `emo-text-layer` の縮退表（R2.4／6.5・`CursorWarnGuard`）の改訂と追随（design・境界節・steering）。
  - 副作用のうちカーソル/行構造に閉じるもの（`\c[line]` の「`\_l` で行が分割される」規定が `\c` 実装時に成立する形の登記または実装——`\c[line]` 自体の実装状況を要件段階で実測すること）。
  - 決定論テスト（3 方向 × 全語彙・境界値・縮退経路）。
- **Out**:
  - `\f[align]`／`\f[valign]` の実装（M2 文字装飾予約——`\_l` 直後の左寄せリセットと `\f[align]` インデント相互作用は**写像の語彙登記のみ**行い、実装は align 実装 spec が所有）。
  - バルーン縦書きの受口・座標意味論・プロパティ（bvc＝`areka-P0-balloon-vertical-canon` が所有）。
  - `\c`／`\_q` 等クリア系タグ自体の新規実装（`\_l` との相互作用の登記まで）。

## Boundary Candidates

- 語彙解析（`CursorCoord` の受理形拡張＝%・@・負値）と座標解決（書字方向別の軸・原点・符号）の 2 シーム。
- 縮退表改訂（完了 spec 追随）を独立タスクに切ると、is-a 改訂の証跡が単独で残せる。

## Out of Boundary

- バルーン定義キー（`vertical`／`writing_mode`）の解析・共存規則（bvc 所有）。
- 縦書きフォント異体・下線・スクロール矢印（bvc が語彙登記・実装は各追跡先）。

## Upstream / Downstream

- **Upstream**:
  - `areka-P0-emo-text-layer`（完了）——縮退表の現行正典。本 spec が改訂する。
  - `areka-P0-balloon-vertical-canon`（bvc・旧 W6.95〔新 W11〕）——⑴ R3 で座標意味論（origin/wordwrappoint/validrect）を決定論テストで固定済みにする ⑵ **origin クランプ正準の撤去**（bvc 討議 #2 裁定・Requirement 3.10）により「実際の 1 列目」＝「宣言された `origin.x` の列」が常に一致＝本 spec の `\_l[0,0]` 着地定義（SC15）が一意 ⑶ 縦書きフィクスチャ（`vertical,1` 版）を供給。**bvc 完了後に着手するのが自然**。
  - SSP 2.8.83 ライブ ukadoc（`\_l` の縦書き節は 2.8.83 で追加＝**ukadoc-mcp スナップショット（2.8.80）に縦書き節は無い**。正典参照はライブで行うこと。bvc requirements.md の SC8・SC9・SC13〜SC15 が関連疑義）。
- **Downstream**:
  - M2 文字装飾（`\f[align]` 系）——`\_l` との相互作用写像を本 spec の登記から引く。
  - `emo2-conformance-e2e`（W7）——emo2 は縦書き・`\_l` の未実装語彙に依存しない見込み（要件段階で emo2 スクリプトの `\_l` 使用実態を grep 確認）＝ e2e を**ブロックしない**。

## Existing Spec Touchpoints

- **Extends**: `areka-P0-emo-text-layer`（縮退表改訂＝完了 spec の正典改訂・追随義務あり）。
- **Adjacent**: `areka-P0-balloon-vertical-canon`（bvc・R4 が本 spec を追跡先として語彙登記・二重実装禁止）。`areka-P0-balloon-canon-residue`（M2 ゲート・収載範囲は系列解決と表示寿命＝本 spec と非交差）。

## Constraints

- Rust 2024。主接触 crate は `areka-emo-text`（`layout.rs` 764 行・`layout_cursor_tests.rs` 670 行・`draw.rs` **974 行＝1,000 行番人まで残 26 行**——draw.rs への追記は避け兄弟ファイルへ・2026-08-27 実測）。
- ウェーブ配置: **M2 解禁ゲート**（M1 では着手しない・開発者裁定で前倒し可）。bvc が縦書き `\_l` の非互換を既知として登記した状態で M1 を閉じる——emo2 非依存のため e2e を妨げない。
- 決定論テスト必達（実 DPI・実 GPU・実窓を要しない形）。縮退経路も含め全語彙を檻に入れる。

---

> **📌 2026-09-02 棚卸⑫（W12 裁定枠 A 候補＝挙動バグ級）**——アンカー再測定: `layout.rs` :656-670（ゲート本体 :658）・:684 `Relative`・:679→**:680**（variant）・:453-454・:305-311・:611-621（`VerticalRl` 腕は :619-624）・:601-610・`layout_cursor_tests.rs` 670 行/13 本/`HorizontalTb` 22 箇所＝**逐語一致**。**変化 2 点**: ⑴ `draw.rs` **974→980 行（残 20）**＝bvc PR#124 で +6・番人の例外表に不在＝1 行超過で赤。本 spec は draw.rs に追記しない（兄弟ファイル）。⑵ 出典 bvc research §3.5／§7 R-1 のパスは **`.kiro/specs/completed/areka-P0-balloon-vertical-canon/`**（08-29 アーカイブ済み）。
> **現行 main の本物の非互換を構造で裏取り**: カーソル X は `layout.rs:453` で常に `region.left()` 起点の**増加**方向、列送りは `:309` の `(start.1, start.0, -1.0)`＝**減少**方向、列矩形は `:620` `left: block_pos - font_height`＝`block_pos` が列**右端**→ `vertical_rl` で `\_l[0,0]` が 1 列目に着地しない。`vertical_lr` は `:310` の `+1.0` で整合。**縦書き `\_l` のテスト被覆は 0 本**（檻に一切かかっていない）。
> 前提: bvc 完了 ✅・M2 ゲート（開発者裁定で前倒し可）・ライブ ukadoc 2.8.83。**W12 同居候補（e2e／channels／toolkit）と共有ファイル 0**（実測）。ウェーブ番号整数化＝本文の W6.95→**W11**・W7→**W12**。分割禁止裁定は不変（Boundary Candidates はタスク分割）。要件定義は Opus で足りる見込み。

