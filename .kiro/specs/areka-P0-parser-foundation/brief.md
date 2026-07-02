# Brief: areka-P0-parser-foundation（本坑 / main・M1 M-boot / parser トラック共通基盤）

> **種別**: 本坑（main）。M1 `areka-P0-emo2-boot` の **parser トラック共通基盤**（並行・単体テスト可・host 不要・依存無し＝即着手可）。
> **経緯（2026-07-02）**: 旧 `areka-P0-balloon-parse`（requirements/design/検証まで完了）は**開発リジェクト**し、本 brief へリネーム・再出発（開発者判断）。設計ディスカッションで「charset デコードは**全パーサー対象で同一仕様**・KV 読み込みは**surface 読み込み以外の全パーサーで論理構造が同一**」と確定し、balloon 単体に閉じた spec 構成が根本から不適切になったため。旧成果物は削除（git 履歴に保全）・引き継ぐべき知見は下記 §知見 に集約。
> **規律**: 過剰・予測実装は禁止。正典は ukadoc（**ukadoc MCP サーバーを積極参照**）・emo2 fixture は最小適合サンプル。

## Problem

`areka-parsers` にこれから増える各パーサー（balloon／shell／package／…）が共通で必要とする土台が存在しない:

1. **charset デコード層**: 全パーサー対象ファイルは冒頭の `charset,文字コード` 行で自身のエンコードを宣言し、**その宣言に従ってファイル全体を読み込み直す**必要がある（見落とすと Shift_JIS ゴーストが文字化け）。仕様は全対象ファイルで同一。
2. **KV 読み込み層**: surface 読み込みパーサー（`surfaces.txt` のセクション構造）以外は、`key,value` フラット行という**全く同じ論理構造**。

各 spec が個別実装すると多重実装になる（balloon/shell/package で三重）。

## Desired Outcome

`areka_parsers` にパーサー共通基盤（2モジュール）が確立し、単体テストで pass:

- **`decode`**: `&[u8]` →（冒頭 ASCII プリスキャンで `charset` 行検出）→ 宣言エンコードで全体デコード → `String`。純粋関数・ファイル I/O なし（バイト列は呼び出し側が渡す）。
- **`kv`**: デコード済み文字列 → 素朴な KV マップ。キーの既知/未知を**一切分類しない**。

後続パーサー spec（balloon/shell/package）は「マップから自分のキーを引いて型付け・解決する」**薄い固有層のみ**になる。

## 知見（引き継ぎ・旧 balloon-parse の成果より）

### charset（ukadoc 確認済み 2026-07-02）

- 仕様は `descript_balloon`／`descript_ghost`／`descript_headline`／`descript_install` で**文言まで同一**:「旧環境互換なら Shift_JIS、それ以外は UTF-8 推奨。省略時は OS 標準または SSP 国際化設定」。SHIORI3 プロトコル側も「Charset は最初の行、少なくとも非 ASCII 行より前が望ましい」→ **冒頭 ASCII プリスキャン**方式は正当（charset 名は ASCII ゆえ実エンコードに関わらず走査可能）。
- 省略時既定（SSP は OS/設定依存）を areka でどう定めるかは要件判断（候補: UTF-8 既定＋寛容フォールバック・破綻させない）。
- **`encoding_rs` 導入は開発者承認済み**（(B) フル対応。areka-parsers「追加依存ゼロ」原則からの**意図的逸脱**）。

### KV パーサー（設計ディスカッション確定）

- **素朴なマップで良い**（行を `split_once(',')` → `HashMap<String,String>` 相当）。キーの既知/未知の分類・専用スロット・未知行コレクションは**過剰実装**（旧設計のリジェクト理由の一つ）。
- 同一キーは後勝ち・空行/分割不能行スキップ・CRLF/LF・BOM・前後空白は寛容・値は文字列のまま保持（数値化・符号解釈は各 spec の固有層）。
- 順序保持は不要（設定マップ。sakura スクリプト＝順序必須の列とは別物ゆえ sakura パターンの機械的踏襲は不適）。

### 層構造（確定）

- `decode`（**全パーサー共通・例外なし**。sakura 辞書系も含む）→ `kv`（**surface 以外の全パーサー共通**）→ 各 spec 固有解決（キー写像・型付け・優先度解決）。
- sakura 規律は踏襲: `Result` 無し寛容・panic しない・不透明 NewType＋read-only アクセサ・`#[non_exhaustive]`・最小派生・`tracing` のみ・in-source テスト・公開パス経由の契約固定。

### balloon 固有知見（後続 balloon spec＝固有層が消費。旧 requirements/design はリジェクト済みだが以下は有効）

- **参照優先度（開発者是正・最重要）**: `balloonsXXs.txt`／`balloonkXXs.txt`（サーフェス別テーブル＝**起点・第1参照**）→ `descript.txt`（共通設定・第2参照）→ 内部既定値（第3参照）。「base descript＋overlay 上書き」という捉え方は**誤り**（リジェクト理由の一つ）。
- **型モデル**: 全フィールド per-surface（共通/サーフェス別を型で区別しない）。各サーフェス側が全フィールドの解決済み確定値を保持し、descript 由来の共通値は両側へ複製（コスト無視可能）。「どれが共通か」の配置判断自体を消す。
- **座標符号**: `validrect`／`wordwrappoint`＝負値は反対端（右/下端）基準。**`windowposition`＝基本位置からの方向調整（y 下が＋/上が−）で反対端基準ではない**（ukadoc 確認済み・混同が最大の落とし穴）。符号付き `i32` 保持で足りる（述語アクセサは過剰）。
- **画像参照**: descript 等に明示ファイル名行は**無い**。命名規約 `balloon{s|k}{ID}.png`（偶数=左向き/奇数=右向き）から導出（ukadoc 確認済み）。モデルはサーフェス種別＋ID のみ保持し導出は下流。
- **emo2 fixture 確定値**（`crates/pilot/examples/shiori-host-32/fixtures/emo2/emo2-kakukaku/`・s0s/k0s とも実データ vendored 済み）:
  - sakura（s0s 起点）: `windowposition(266,-129)`・`wordwrappoint.x=-49`（descript の -34 を起点が上書き）・`validrect(top=46,bottom=-56,left=36,right=-44)`・`arrow0(15,90)`/`arrow1(15,-110)`・font "Yu Gothic UI" h=28・font.color(0,0,0)・anchor.font.color(180,40,40)。
  - kero（k0s 起点）: `windowposition(-190,-75)`・`validrect(top=40,bottom=-70,left=24,right=-48)`・`arrow0(9,54)`/`arrow1(9,-125)`・`wordwrappoint.x=-34`（k0s に無し＝**descript フォールバックの生き証人**）。
- fixture テストはリテラル直書き＋採取元の正本ファイル名・行コメント義務（クレート跨ぎ `include_str!` の脆さ回避）。

## Scope

- **In**: `areka_parsers` の `decode`（charset 検出＋encoding_rs デコード）／`kv`（KV マップ化）モジュールと単体テスト。emo2 fixture（UTF-8）＋非 UTF-8（Shift_JIS）合成入力での検証。
- **Out**: 各 spec 固有の解決層（balloon/shell/package のキー写像・型付け・優先度解決）。surface セクションパーサー（`surfaces.txt`・kv 非対象、decode のみ利用）。ファイル読み込み I/O（呼び出し側の領分）。

## Boundary Candidates

- charset 行の検出（プリスキャン範囲・書式寛容度）
- 宣言エンコードによる全体再デコード（encoding_rs・未対応/不正宣言時の寛容フォールバック）
- KV 行のマップ化（後勝ち・trim・スキップ規則）

## Out of Boundary

- キーの意味解釈（全て各 spec 固有層の領分）
- sakura スクリプト構文・surface セクション構文

## Upstream / Downstream

- **Upstream**: `areka-parsers`（`sakura` 規律）／ukadoc（charset 正本）。
- **Downstream**: `areka-P0-balloon-parse`（再切り出し・固有層のみ）／`areka-P0-shell-parse`／`areka-P0-package-mount`（いずれも本基盤に依存）／surface parser（decode のみ依存）。

## Existing Spec Touchpoints

- **Extends**: `areka-parsers`。
- **Adjacent**: `areka-P0-shell-parse`／`areka-P0-package-mount`（従来「並行安全」だったが、本基盤が**先行依存**になる＝着手順序に注意）。

## Constraints

- Rust 2024・`encoding_rs` 追加（承認済み・唯一の外部依存追加）・`tracing` のみ・`Result` 無し寛容・`#[non_exhaustive]`・過剰実装禁止。
- 正典は ukadoc（**ukadoc MCP を積極参照**）。不確実は質問。
