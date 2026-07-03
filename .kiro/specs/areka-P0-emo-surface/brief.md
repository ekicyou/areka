# Brief: areka-P0-emo-surface

> **種別**: 本坑（main）。⑥ emo（render engine）トラックの基盤ユニット（M-boot・下流に emo-text-layer）。
> **調査日**: 2026-07-03（mock-shell/wintf コード深掘り＋ukadoc 正典）。

## Problem

M1 の描画原則は「**シェルもバルーンも同一の surface 合成**」（統一エンジン・記憶 areka-unified-shell-balloon-graphics）だが、現状の areka（mock-shell）は base.png 一枚＋テキストバルーンのハードコードであり、**parsers が生む型付き Shell モデル（surface／element／入れ子参照）を合成表示するエンジンが存在しない**。ここが埋まらないと seriko（SERIKO ループ）も emo-text-layer も載る土台がない。

## Current State

- **mock-shell 実体**: `crates/areka/src/main.rs` — 単一 `BitmapSource`（base.png）＋ `HitTest::alpha_mask()` 子＋Typewriter バルーン。element 合成・surface 選択なし。
- **wintf 描画基盤（WUC 移行済）**: `BitmapSource`→WIC 読込→`Visual`/`VisualGraphics`→WUC `SpriteVisual`。**入れ子 Visual 合成は実装済み**（ECS `ChildOf` → `visual_hierarchy_sync_system` が `ContainerVisual::InsertAtBottom` へ鏡映・`visual_sync.rs:25`）。
- **変換行列の現状**: 現行 wintf の visual property 同期は **Offset/Scale 分解プロパティのみ使用**（`visual_property_sync`）。D2D 側は `SetTransform(Matrix3x2)` が command sink に存在。**WUC `Visual.TransformMatrix` 直結の可否は design で要検証**（行列を内部表現とする本ユニットの核心制約）。
- **入力モデルは完成済み**: `areka_parsers::shell::parse(&str)->Shell`（Surface{id, elements[layer,path,x,y], collisions, animations}・appends・aliases の 13 型）✅／`areka_parsers::balloon`（幾何＋フォント）✅／`package::MountModel`（shell dir 解決）✅。
- **バルーン枠画像の読込は未実装**: balloon window はテキストのみ。`balloons*.png` を読む経路は存在しない（機構は BitmapSource がそのまま使える）。

## Desired Outcome

型付き Shell/balloon モデルを入力に、**surface（base＋element 合成・入れ子参照可・配置＝行列内部表現）としてシェルとバルーン枠を同一エンジンで表示**できる。

**✔ 観測（単一 pass/fail）**: 専用 example が emo2 fixture を読み、**surface0（base＋element 合成）とバルーン枠（`balloons*.png` を surface として読込）を統一合成で表示**する。window-placement 完了を待たない（既存 mock-shell 級の窓生成を example 内で自前使用＝別ユニット先行不要）。

## Approach

1. **Surface entity モデル**: `Shell` の Surface → ECS entity（子= element ごとの `BitmapSource` visual・**layer 順で InsertAtBottom**）。element 合成は wintf の入れ子 Visual 合成を lift。
2. **配置＝行列内部表現**: element/surface 配置を Matrix3x2 で持つ（x,y は単位行列の特例）。WUC への適用は **(a) `Visual.TransformMatrix` 直結 or (b) Offset/Scale/Rotation 分解**を design で検証・選択（emo2 は平行移動のみ使用だが構造は行列＝最初から）。
3. **入れ子サーフェス参照**: element＝{画像 path | 他 surface id 参照} の再帰 spawn（構造のみ・emo2 は平面 overlay）。
4. **バルーン枠＝surface**: `balloons*.png`/`balloonk*.png` を surface として同一機構で読込・表示。**balloon dir は fixture パス直指定**（ベースウェアのバルーン選択・所在解決は ghost 層の後続領分＝疎結合化）。
5. **透過規則（ukadoc）**: emo2 fixture の `seriko.use_self_alpha` と `.pna` 有無を design で確認し、**emo2 が使う腕だけ実装**（未使用の腕は型シームのみ）。
6. **surface 指令 API**: 「surface id を切り替えて表示」の命令適用口を持つ（M-boot は静的表示＋指令適用。SERIKO ループ駆動・channel 契約は seriko-engine 時に確定）。

## ukadoc 正典要点（design の前提事実）

- **element 合成**: base surface PNG に `element0..N` を **0→N 順にマージ**して一枚として扱う。合成メソッド subset: `overlay`（単純重ね）／`overlayfast`／`replace`／`base`／`reduce`／`asis` 等（emo2 使用分のみ実装・M1 スコープ表準拠）。element 座標＝左上原点オフセット。
- **透過**: 既定＝**左上ピクセル＝キーカラー透過**。`seriko.use_self_alpha,1`＝PNG α＋`.pna`（グレースケール・黒=透明）併用可、`full`＝α のみ。優先: PNG α ＞ .pna ＞ キーカラー。
- **関連 descript キー**: `seriko.use_self_alpha`／`seriko.paint_transparent_region_black`（半透明域の描色規則・既定は .pna 系=1/α系=0）／`seriko.dpi`。
- **バルーン画像**: `balloons0/1.png`（本体側・左右）＋`balloonk*.png`（相方側）。α 規則は surface と同一（.pna 可・SSP 系）。最小表示は balloons0 一枚で成立。

## Scope

- **In**: Surface entity 合成（base＋element layer 順・入れ子参照構造）／行列内部表現＋WUC 適用方式の確定／キーカラー・α・pna のうち emo2 使用腕／バルーン枠画像の surface 読込（fixture 直指定）／surface 指令適用口／専用 example（観測）。
- **Out**: SERIKO アニメ再生・interval・MAYUNA（**seriko-engine**／seriko-loop・mayuna-compose）／テキスト描画・scroll（**emo-text-layer**）／collision→region 写像（**collision-geometry**）／バルーン所在のベースウェア解決・ユーザ選択／`\q` 選択肢・二人立ち surface（増分）／emo2 未使用の合成メソッド・透過腕の実装。

## Boundary Candidates

- 合成コア（Shell モデル→Visual ツリー写像・純粋変換）と表示結線（window への装着）の分離
- 画像解決（path→BitmapSource）と合成構造（layer/入れ子）の分離
- 指令適用口（surface id switch）＝将来の seriko→emo channel 契約の片側

## Out of Boundary

- Window entity の生成・既定位置・ドラッグ（**window-placement**＝Window 層。本ユニットは Visual ツリー層のみ所有。統合時は window-placement の窓の下へ載る）。

## Upstream / Downstream

- **Upstream**: `areka-P0-shell-parse` ✅・`-balloon-parse` ✅・`-package-mount` ✅（型付き入力）／wintf WUC 基盤 ✅（Visual 入れ子・BitmapSource・AlphaMask）／dola ✅（M-boot では時計のみ・ループは M-life）。
- **Downstream**: `areka-P0-emo-text-layer`（本 surface の上の文字層）／`areka-P0-seriko-engine`（surface 指令の発行者）／`collision-geometry`・`choice-render`・`dual-window`（増分）。

## Existing Spec Touchpoints

- **Extends**: `completed/areka-mock-shell`（描画素材の donor・main.rs パターン lift）。
- **Adjacent**: `areka-P0-window-placement`（**同じ `crates/areka/src/main.rs` 起点＝並行着手時はファイル衝突注意・順次推奨**。境界: Window entity=placement／Visual ツリー=本ユニット）。
- 既知ドリフト解消: `balloon/model.rs:6` doc コメントの旧名 `text-layer`/`surface-engine` 参照を本ユニット着手時に追随修正（記憶 areka-engine-names）。

## Constraints

- Rust 2024・`windows` 0.62.2・tokio 禁止。**WUC を触る graphics は UI スレッド固定**（DispatcherQueue 親和性・MTA＋`DQTAT_COM_NONE`＝記憶 areka-wuc-runs-on-mta-thread）。
- 最小実装＋薄い拡張シーム（emo2 使用分のみ・行列/入れ子/統一エンジンの**構造**は最初から）。
- 正典は ukadoc・emo2 fixture は最小適合サンプル（書式の聖典としない）。
