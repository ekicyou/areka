# Brief: areka-P0-balloon-parse（本坑 / main・M1 M-boot / parser トラック）

> **種別**: 本坑（main）。M1 `areka-P0-emo2-boot` の **parser トラック**（roadmap「parsers＝並行・単体テスト可・host 不要」）。**依存: `areka-P0-parser-foundation`（charset デコード＋KV 共通基盤・2026-07-02 完了）が先行充足済**。foundation 完了後は `shell-parse ∥ balloon-parse ∥ package-mount` で並走安全（同クレート別モジュール・非衝突）。
> **規律**: emo2 が実際に使う機能のみ実装（過剰・予測実装は禁止）。拡張は型の `#[non_exhaustive]` シームのみ残す。正典は ukadoc（`descript_balloon`）・emo2 fixture は最小適合サンプル。

## Problem

emo2 のバルーン設定（balloon `descript.txt` ＋ 画像別 `balloonsXXs.txt`/`balloonkXXs.txt`）を**バルーンモデル**へ解析する parser が存在しない。下流の `text-layer`（バルーン文字を engine 上に被せる層）と `surface-engine`/render（バルーン枠 surface の配置）が消費する、**幾何＋フォント＋3段優先度解決済み**のモデルの生成源が要る。foundation は素朴 KV マップ（`BTreeMap<String,String>`）までしか担わず、**バルーン固有のキー写像・座標符号解釈・3段参照優先度解決は未所有**（foundation design「Out of Boundary: balloon の 3 段参照など」）。

## Current State（調査済み・接ぎ木先）

- **`areka-parsers` クレート**（`crates/areka-parsers/`）: `sakura` パターン確立済み ── `pub fn parse(...) -> Model`（**`Result` 無し・寛容パス**・未知は吸収）、値型は **NewType＋opaque inner＋read-only accessor**、enum は `#[non_exhaustive]`、in-source `#[cfg(test)]` テスト。foundation で **`charset::decode(bytes, DefaultEncoding) -> String`** ＋ **`kv::parse_kv(&str) -> BTreeMap<String,String>`** が確立（2026-07-02 完了）。本 spec は同クレートへ **`balloon` モジュール**を追加し、foundation 2 段（charset→KV）の出力を消費する固有層を書く。
- **emo2 fixture**: `crates/pilot/examples/shiori-host-32/fixtures/emo2/emo2-kakukaku/{descript.txt, balloons0s.txt, balloonk0s.txt}`（全 UTF-8。foundation の validation テストが既に同 fixture を採取元にしている）。
  - `descript.txt`（既定層）: `charset,UTF-8` ／ `origin.x,0`/`origin.y,0` ／ `wordwrappoint.x,-34`/`wordwrappoint.y,0` ／ `validrect.{top,bottom,left,right}` ／ `font.name,Yu Gothic UI`/`font.height,28`/`font.color.{r,g,b}` ＋ cursor/anchor/number/marker/sstp/arrow/communicatebox 群。
  - `balloons0s.txt`/`balloonk0s.txt`（画像別上書き層）: `windowposition.{x,y}` ／ `wordwrappoint.x` ／ `validrect.*` ／ `arrow{0,1}.{x,y}` ／ `number.*` ／ marker 群（charset 行なし＝foundation 既定エンコードで通る）。
- **出力モデル型は未存在**: バルーンモデル（幾何＋フォント＋解決済み値）は codebase に無い＝**本 spec で定義**。

## Desired Outcome

emo2 の 3 ファイル（balloon `descript.txt` ＋ 指定画像の `balloonsXXs.txt`/`balloonkXXs.txt`）から、**3段参照優先度（画像別＞descript＞既定）を解決したバルーンモデル**（バルーン配置 `windowposition`・文字描画原点 `origin`・折返し `wordwrappoint`・有効矩形 `validrect`・フォント `font.*`）を生成でき、emo2-kakukaku fixture で pass。純粋関数・単体テストのみで観測可能（host 不要）。

## Approach

`areka-parsers` に `balloon` モジュールを追加し、`sakura`/foundation パターンを踏襲。**emo2 最小 feature set**（`doc/emo2-conformance-scope.md` 由来）:

- **入力は「デコード済み文字列」または「foundation KV マップ」を受ける純粋関数**。ファイル所在解決・どのバルーンフォルダを使うかは**受けない**（baseware 共有・ユーザ選択ゆえ ghost/package 領分＝記憶 areka-ghost-boot-descript-not-install）。呼び出し側が 3 ファイルのバイト列/パスを渡す。
- **3段参照優先度の解決**: 画像別（`balloonsXXs.txt`/`balloonkXXs.txt`）のキー ＞ balloon `descript.txt` のキー ＞ 組込み既定。画像別と descript の **ファイル間マージ**を本 spec が所有（foundation の後勝ちマップを 2 層に重ねる）。「既定」層の所有（parser 定数 vs 消費側）は要件で確定（Boundary Candidates 参照）。
- **座標符号解釈**: foundation が明示的に非所有とした「座標符号解釈」を本 spec が所有。SSP 慣行の負値＝反対辺からのオフセット（例 `validrect.bottom,-56`・`wordwrappoint.x,-34`）を型付き幾何へ写像。
- **モデル化するキーは emo2-boot 必須の幾何＋フォント subset**: `windowposition`・`origin`・`wordwrappoint`・`validrect`・`font.{name,height,color}`。**choice/link/scroll 系（cursor・anchor・number・arrow・sstpmarker/message・onlinemarker・communicatebox）は M1 未実装ゆえモデル化しない**（roadmap 規律「choice/link/text-effect は実装しない」）。これらのキーは foundation マップに残置されるだけで本層は消費しない（寛容passthrough）。
- 未知トークンは寛容に吸収。モデル型は `#[non_exhaustive]` で将来のキー拡張シームのみ残す。

## Scope

- **In**: `areka_parsers::balloon` モジュール。バルーンモデル型定義（幾何＋フォント）。foundation（charset→KV）出力の消費。**3段参照優先度解決（画像別＞descript のファイル間マージ）**。**バルーン座標の符号解釈**（負値＝反対辺オフセット）。emo2-kakukaku fixture ベースの in-source テスト。
- **Out**: バルーンフォルダの所在解決・どのバルーンを使うかの選択（ghost/package 領分・baseware 共有）。charset デコード・KV マップ化（foundation 領分）。文字描画・バルーン枠 surface 合成（`text-layer`/`surface-engine`）。choice/link/scroll 系キー（cursor/anchor/number/arrow/marker/sstp/communicatebox）のモデル化・挙動（M1 未実装）。さくらスクリプトのバルーン系タグ（`\b`/`\_b`/`\q`）解析（`sakura-parse` 領分）。

## Boundary Candidates

- foundation 出力（KV マップ）→ バルーン既定層モデル（descript.txt 由来）
- 画像別上書き層（balloonsXXs/balloonkXXs）→ 既定層への 2 層マージ（後勝ち・キー単位）
- バルーン座標の符号解釈（負値＝反対辺オフセット）＝幾何写像
- フォントモデル（name/height/color）
- 「既定（組込みデフォルト）」層の所有 ── parser 内定数として持つか、未指定は None で消費側に委ねるか（要件で確定）

## Out of Boundary

- charset/KV 前処理（foundation）。
- 描画・surface 合成・文字レイアウト（下流エンジン）。
- 他 parser（shell/package）の領分・バルーン所在解決（ghost/package）。
- さくらスクリプトのバルーン操作タグ（sakura-parse）。

## Upstream / Downstream

- **Upstream**: `areka-parsers`（`sakura` パターン＋`areka-P0-parser-foundation` の `charset`/`kv`・完了済）／emo2-kakukaku fixture。
- **Downstream**: `areka-P0-text-layer`（バルーン文字層・`origin`/`wordwrappoint`/`validrect`/`font` を消費）／`areka-P0-surface-engine`・render（`windowposition`＝バルーン枠 surface 配置）。

## Existing Spec Touchpoints

- **Extends**: `areka-parsers`（`areka-P0-sakura-parse` 作成・`areka-P0-parser-foundation` 拡張のクレート）。
- **Adjacent**: `areka-P0-shell-parse` / `areka-P0-package-mount`（同クレート別モジュール・**非衝突・並走安全**。ただし `lib.rs`/`Cargo.toml` の共有シームはマージ順に留意）。

## Constraints

- Rust 2024・std 中心・追加依存は foundation の `encoding_rs`（クレート経由で利用・新規外部依存は足さない）＋ workspace `tracing` のみ。**`Result` 無しの寛容パース**・値型は NewType＋opaque＋accessor・enum は `#[non_exhaustive]`。
- **emo2 実物 fixture（emo2-kakukaku）で検証**・**過剰実装禁止**（emo2-boot が使う幾何＋フォントのみ・2 例目の実物が要求するまで抽象を足さない）。
- 不確実な balloon/descript.txt 仕様は推測せず ukadoc（`descript_balloon`）／`doc/emo2-conformance-scope.md`／fixture を正とし、無ければ質問。
