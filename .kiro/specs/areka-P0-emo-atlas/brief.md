# Brief: areka-P0-emo-atlas

> **種別**: 本坑（main）。⑥ emo トラック直列チェーン **1/3**（emo-atlas → emo-compose → emo-present。旧 `areka-P0-emo-surface` の 2026-07-03 粒度分割）。
> **方針正本**: 合成は emo 自前・アトラス転写・1枚物（記憶 areka-emo-own-compositor-atlas／roadmap emo 節）。

## Problem

emo の自前合成（wintf Visual 合成非依存・1枚物ビットマップ生成）には、その素材基盤——**element 画像を正規化してアトラスに焼き付け、転写可能な形で保持する層**——が必要だが存在しない。ここが純粋・単体テスト可能な形で立たないと、合成コア（emo-compose）が土台なしになる。

## Current State

- wintf に WIC 読込（`BitmapSource`→PBGRA）は存在するが、**複数画像の一括デコード・透過正規化・トリミング・アトラス焼付**の機構は無い。
- 入力モデル完成済み: `areka_parsers::shell`（element の `ElementPath`）✅・`balloon` ✅・`package::MountModel`（shell dir 解決）✅。
- **クレート調査済み（2026-07-03）**: packing は **`rectangle-pack`（本命）**——zero-dep・MIT/Apache・静的バッチ packing・複数 bin（頁）対応・活発維持。padding は非内蔵＝**矩形を 1〜2px 拡げて渡す自前ラップ**（自明）。対抗 **`rect_packer`**（zero-dep・MIT・padding 内蔵だが単頁 API＝複数頁は DIY）。**棄却**: `texture_packer`（`image` クレート強制依存＋休眠）・`guillotiere`/`etagere`（動的アロケータ＝毎フレームグリフ用途・bake-once には過剰＆前者は停滞）・`crunch`（回転サポート不要）。

## Desired Outcome

emo2 の element 画像群（shell＋balloon）が、**透過正規化済み premultiplied BGRA として、α=0 領域を除外したトリム矩形でアトラスへ焼付**され、合成側が「element path → (頁, UV 矩形, トリムオフセット, 原寸)」を引ける。

**✔ 観測（単一 pass/fail・表示不要・純粋層）**: 単体テストで (a) **トリミング**——例: 100×100 の画像で有効（α>0）領域が 10×10 なら、アトラスには 10×10 が焼かれ、**トリムオフセットと原寸が正しく記録**される（全透明画像→空エントリ＝転写スキップ扱い）／(b) 透過正規化（キーカラー/`.pna`/α）の画素一致／(c) emo2 fixture 画像群の一括焼付が頁内に padding 込みで重なりなく収まる。

## Approach

1. **デコード**: 既存 WIC 経路を流用（新規依存ゼロ）。単体テストは COM init（`CoInitializeEx`）が必要な点に注意（wintf の既存テスト慣行に合わせる）。※ `image` クレート（pure Rust・headless 容易・ただし依存 15+ 本）への差替えは**採らない**（最小依存方針）——デコード層を trait で薄く切り、将来差替え可能にだけしておく。
2. **透過正規化（挿入時一度だけ）**: `seriko.use_self_alpha`（0=キーカラー/1=PNG α＋`.pna`/full）を解釈し **premultiplied BGRA へ統一**。優先: PNG α ＞ `.pna` ＞ キーカラー（左上ピクセル）。emo2 fixture が使う腕を design で確認し、その腕のみ実装（他は型シーム）。
3. **αトリミング**: 正規化後に α>0 の**タイトな有効矩形**を走査し、`trim_offset(x,y)`＋`trimmed(w,h)`＋`original(w,h)` を記録。トリム矩形のみをアトラスへ焼付。**全透明→空エントリ**（合成側は転写スキップ）。
4. **packing**: `rectangle-pack` で静的バッチ packing（**新規依存＝開発者承認必要**・encoding_rs 前例に倣い brief で申請→design で確定）。頁サイズ 2048（必要時 4096）・複数頁・**padding 1〜2px は矩形拡張の自前ラップ**（bleed 防止）・同一 path の重複排除（焼付前に path→entry を索引）。
5. **アトラス表**: `AtlasKey(path)` → `AtlasEntry{page, uv_rect, trim_offset, original_size}`。頁本体は premultiplied BGRA バッファ（CPU 保持が既定・GPU 化は emo-compose のバックエンド選定に従う）。

## クロスユニット契約（後続を詰ませない事前考慮・2026-07-03 fixture 実測反映）

- **マニフェスト導出は本ユニットが所有**: 「Shell/balloon モデル→焼付対象パス一覧」の列挙器を持つ。対象＝**全 surface の element パス**（emo2 は base 画像も `element0,overlay,surface0.png,0,0` と element 自己参照する流儀＝「base は別枠」と設計しない）＋**bind アニメーション pattern が参照する surface の element 画像**（間接参照・surface1000 系）＋balloon 画像。サブディレクトリパス（`CityPop/`・`purple/` 配下）は ElementPath のまま素通し。surfaces.txt 未定義の file-only surface（`surfaceN.png` 直参照）は ukadoc 上有効＝**シームだけ設ける**（emo2 は全 64 surface 定義済みで不要）。
- **AtlasEntry 契約は emo-compose と共有の正本**: `AtlasKey(path)` → `{page, uv_rect, trim_offset, original_size}`＋頁バッファ（premultiplied BGRA・stride 明示）。この形が emo-compose の転写入力＝**design 冒頭で両ユニット共通の型として確定**（compose 側で再定義しない）。
- **正規化パラメータは入力で受ける**: `use_self_alpha`／`paint_transparent_region_black` は shell descript（parsers 済）由来の設定として注入（アトラスが descript を読みに行かない＝層分離）。
- **emo2 fixture 実測（2026-07-03）**: `seriko.use_self_alpha,1`・**`.pna` ファイル無し**→ **主実装腕＝PNG 自身の α チャンネル**（キーカラー腕・pna 腕は型シームのみ）。charset,UTF-8。

## 設計指示・注意点

- **premultiplied 一貫性**: 正規化→焼付→（下流の）転写の全段 premultiplied。straight α 混入＝にじみ/暗縁の典型バグ源。
- **トリムの意味論**: 合成側は「element 配置座標＋trim_offset」で転写する契約——**トリムは配置を変えない**（見た目等価・メモリ/転写量だけ減る）ことをテストで固定。
- **padding の帰属**: UV は padding を含まない実矩形を指す（サンプリング bleed は padding 画素が防ぐ）。
- **決定性**: 同一入力→同一 packing 結果（`rectangle-pack` は決定的）。golden テストを安定させる。
- **頁あふれ**: 1頁に収まらない場合は複数頁へ自然分割（emo2 規模では起きないが構造は最初から）。
- **バルーン画像も同一機構**（`balloons*.png`/`balloonk*.png`・α規則は surface と同一）。

## Scope

- **In**: WIC デコード結線（trait 薄切り）／透過正規化（emo2 使用腕）／αトリミング＋オフセット記録／`rectangle-pack` packing（承認申請）＋padding 自前ラップ＋複数頁＋重複排除／アトラス表 API／単体テスト群。
- **Out**: 合成（**emo-compose**）／表示（**emo-present**）／SERIKO 再生（seriko）／`image` クレート導入／動的アトラス（毎フレーム挿入）／emo2 未使用の透過腕の実装。

## Boundary Candidates

- デコード＋正規化（画素変換・純粋）／トリミング（走査・純粋）／packing＋表（配置・純粋）の三層——いずれもオフスクリーン単体テスト可。

## Out of Boundary

- 合成メソッド・行列・ツリー展開（emo-compose）／wintf との接続（emo-present）。

## Upstream / Downstream

- **Upstream**: `areka-P0-shell-parse` ✅（ElementPath）・`-balloon-parse` ✅・`-package-mount` ✅（dir 解決）／wintf WIC 経路 ✅。
- **Downstream**: `areka-P0-emo-compose`（アトラス表＋頁バッファの唯一の消費者）→ `emo-present` → `emo-text-layer`。seriko-loop（M-life）の毎フレーム再合成はこのアトラスの転写性能に乗る。

## Existing Spec Touchpoints

- **Extends**: なし（新設層）。**Adjacent**: `areka-P0-window-placement`（別層・非衝突）／wintf `bitmap_source`（WIC 経路の参照元・不改変）。

## Constraints

- Rust 2024・tokio 禁止。**新規依存 `rectangle-pack` は開発者承認必要**（zero-dep・MIT/Apache 確認済・fallback は `rect_packer`）。
- 最小実装＋薄い拡張シーム（emo2 使用腕のみ・トリム/複数頁/重複排除の**構造**は最初から）。
- 正典は ukadoc（`use_self_alpha`/`.pna`/キーカラー規則）・emo2 fixture は最小適合サンプル。
