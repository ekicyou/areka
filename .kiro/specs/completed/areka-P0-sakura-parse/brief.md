# Brief: areka-P0-sakura-parse

> ロードマップ M-boot・parsers トラックの実装ユニット（`.kiro/steering/roadmap.md` L86）。
> 実物スコープの正本は [doc/emo2-conformance-scope.md](../../../doc/emo2-conformance-scope.md) §3「さくらスクリプト（sakura-script の M1 実需）」。
> 規律: 実装ファースト・最小実装＋薄い拡張シーム（roadmap「実装規律」）。spec を産まない 1 ユニット＝1 かたまりの動く振る舞い。

## Problem

areka（x64 最小 SSP 互換ベースウェア）が emo2 を「そのまま」会話させるには、SHIORI 応答の `Value:` に載って返る**さくらスクリプト本体**を、シェル／バルーン演出エンジンが実行できる形へ解析する層が要る。現状 `crates/areka` にさくらスクリプトの parser は存在しない（`crates/*sakura*` なし）。この層が無ければ、host-32 が `OnBoot` の Value を受領できても**誰も喋れない・誰も動けない**。

## Current State

- host-32 SHIORI 契約チェーンは completed（`areka-P0-shiori-com` → protocol → split → reference）。Value（さくらスクリプト文字列）を x64 側へ運ぶ経路は M-boot で結線される。
- `areka-mock-shell` が縦書き Typewriter バルーン＝動くレンダリング素材を提供済み（下流の描画先）。
- **欠落**: Value のさくらスクリプトを構造化された実行可能命令へ変換する parser が無い。emo2 が使うタグ subset の decode 仕様も未コード化。

## Desired Outcome

emo2 の boot script（および emo2 が実際に使うタグ subset を含む任意のさくらスクリプト）を入力すると、**完全に decode 済みの型付き命令列（構造化 AST）**を返す純粋関数が存在する。下流 `sakura-engine` はこの命令列を**そのまま実行（タイムライン再生）するだけ**で、文字列を二度と解析しない。

**Done（単一 pass/fail）**: emo2 の boot script を期待どおりの命令列へ変換する単体テストが green（host 不要・並走で独立観測可能）。

## Approach

**全ての字句解析・構造化・値デコードを parser が所有する。** さくらスクリプトの string を走査し、emo2 が使うタグ subset（下記）を**意味の確定した型付き命令ノード**へ変換する。下流エンジンが再パースしなくて済むよう、parser 段階で値を正規化・decode しきる:

- `\w[n]` / `\wN`（短縮）/ `\_w[ms]`（絶対 ms）→ **正規化済み待ち時間（Duration）** へ統一 decode。
- `\n[percent]`（割合改行 `\n[150]`=1.5 行）→ **比率値**へ decode。素の `\n` は既定比率。
- `\p[n]` → 話者スコープ。`\s[ID|エイリアス]` → サーフェス指令（**中身は不透明文字列のまま**＝数値前提で parse しない・surface 層へ委譲）。
- `\q[disp,target]` ＋ `\![*]`（選択肢マーカー）→ disp/target を分離した**型付き Choice 命令**。
- `\_l[x,y]`（カーソル絶対位置・em/lh）/ `\e`（終端）/ `\c`（クリア）/ `\-`（終了）→ 各型付き命令。
- `\![move,dx,dy,...,base,base]` → **引数を decode した型付き Move 命令**（`\!` のうち move のみ構造化）。
- `%username` → **システム変数 token**（展開はしない・下流の実行時責務）。
- タグ間のプレーンテキスト → Text run 命令。

**エラー方針＝寛容パススルー**: 未知の `\!` コマンド・想定外タグ・不正トークンは raw/unknown 命令として保持し解析を継続（実ゴーストを止めない互換ベースウェア向き）。move 以外の `\!` は generic command として通す。

**拡張シーム（口だけ残す・実装しない）**: 命令種別を enum/レジストリで開き、M1 未使用タグ（`\b \_b \i \j \& \f[] \x` 等・追加 `\!` 系）は後日 variant 追加で対応可能にする。abstraction は「2 例目の実物」が要求してから。

## Scope

- **In**:
  - emo2 タグ subset（§3）の字句解析と**完全デコード**＝型付き命令列（構造化 AST）の生成。
  - `\w`/`\_w` の待ち時間正規化、`\n[percent]` の割合 decode、`\q` の disp/target 分離、`\![move]` 引数 decode。
  - `\s[]` 中身の不透明文字列保持、`%username` のシステム変数トークン化。
  - 寛容パススルー（未知タグ／不正トークンの raw 保持）。
  - boot script を題材にした単体テスト（host 不要）。
- **Out**:
  - **命令の意味解釈・タイムライン再生・wait 実行**（→ `areka-P0-sakura-engine`）。parser は実行しない。
  - **`%username` の実展開**（実ユーザ名置換は実行時コンテキスト要 → engine/runtime）。
  - **`\s[]` 中身の数値解釈／エイリアス→ID 解決**（→ surface 層・shell-parse）。
  - **`\![move]` の窓移動実行**（→ render/window-placement）。parser は引数を decode するのみ。
  - charset 変換（UTF-8 前提。Shift_JIS は M2 の生態系拡張）。
  - 脳（`.pasta`/`.lua`/budoux/縦書き設定）の解釈（pasta.dll の腹の中・areka 不介入）。

## Boundary Candidates

- **字句層（lexer）**: string → 生タグ／テキスト境界の切り出し。
- **decode 層**: 生タグ → 値正規化済みの型付き命令ノード（待ち時間／割合／choice／move 引数）。
- **命令モデル（型）**: 下流 engine と共有する型付き命令 enum（クロスエンジン I/O 契約の片側）。

## Out of Boundary

- さくらスクリプトの**実行**（タイムライン再生・wait・surface 指令の発行）＝ `areka-P0-sakura-engine`。
- システム変数の**展開**（`%username` 実値置換）＝実行時。
- サーフェス**エイリアス解決**・surface 合成＝ `areka-P0-shell-parse` / surface 層。
- バルーン定義の解析＝ `areka-P0-balloon-parse`。
- パッケージ配置／install.txt 解決＝ `areka-P0-package-mount`。

## Upstream / Downstream

- **Upstream**: なし（純粋関数＝さくらスクリプト string → 命令列）。入力文字列は host-32／conductor 経由で SHIORI Value から到来するが、parser 自体にコード依存は無い（並走安全・依存は M-boot のみ）。
- **Downstream**: `areka-P0-sakura-engine`（さくらスクリプト再生エンジン）が命令列を消費して実行。`conductor` が Value を parser へ渡す。

## Existing Spec Touchpoints

- **Extends**: なし（新規ユニット）。旧 `areka-P0-sakura-script`（completed/履歴）は「全タグ網羅」志向→本ユニットは emo2 実需の**約12タグ＋`\![move]`** へ縮小（rescope サマリ §6）。
- **Adjacent**: 兄弟 parser `areka-P0-shell-parse` / `areka-P0-balloon-parse` / `areka-P0-package-mount`（別境界・重ねない）。`\s[]` の不透明中身は surface 層へ委譲（共有シーム）。下流の命令モデル型は `areka-P0-sakura-engine` と共有（I/O 契約）。

## Constraints

- Rust 2024・`crates/areka` 内のモジュールとして実装（新規クレートは作らない方針が妥当・配置は着手時に確定）。32bit 可搬性を崩さない。
- 純粋・host 非依存・単体テスト可（並走安全ユニット）。
- 入力 charset は UTF-8（emo2）。Shift_JIS は M1 範囲外。
- 最小実装＋薄い拡張シーム。未使用タグ・abstraction は実物 2 例目まで作らない。
- 設計判断の正本は [doc/COMPAT_ARCHITECTURE.md](../../../doc/COMPAT_ARCHITECTURE.md)、実物スコープは [doc/emo2-conformance-scope.md](../../../doc/emo2-conformance-scope.md) §3。
