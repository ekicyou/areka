# Brief: areka-P0-emo-surface

> **種別**: 本坑（main）。⑥ emo（render engine）トラックの基盤ユニット（M-boot・下流に emo-text-layer／seriko-engine の駆動先）。
> **調査日**: 2026-07-03（mock-shell/wintf コード深掘り＋ukadoc 正典）。
> **⚠️ 2026-07-03 開発者方針決定（本 brief の最上位指示）**: サーフェス合成は **wintf の Visual/エレメントモデルに依存しない**。emo 自前の合成コンポーネントが **1枚物の合成済みビットマップ**を生成し、element 画像は**最適化されたアトラス**に貼付して合成する。

## Problem

M1 の描画原則は「**シェルもバルーンも同一の surface 合成**」（統一エンジン・記憶 areka-unified-shell-balloon-graphics）だが、現状の areka（mock-shell）は base.png 一枚＋テキストバルーンのハードコードであり、**parsers が生む型付き Shell モデル（surface／element／入れ子参照）を合成表示するエンジンが存在しない**。

さらに合成の実現方式に本質的制約がある: **DComp/WUC の visual ツリー合成はブレンドモードが限られ**（実質 SourceOver 系）、SERIKO の合成メソッド群（`overlay`／`overlayfast`／`replace`／`interpolate`／`reduce`＝α乗算／`asis`／`blend-*` 群）やキーカラー・`.pna` 透過を**ピクセル正確に写像できない**。visual ツリーに element を並べる方式は将来必ず破綻する。

## 方針決定（開発者指示・正）

1. **自前合成**: surface 合成は emo が所有する自前コンポーネントで行う。Shell モデル → base＋element を layer 順に合成 → **surface 1枚＝合成済みビットマップ1枚**。wintf へは**完成品1枚を渡すだけ**（表示ノードは窓あたり最小限の visual・入れ子 Visual 合成は不使用）。
2. **アトラス**: 子 element 画像は読込時にデコードし、**最適化されたテクスチャアトラス**へ貼付。合成はアトラス領域からの転写（blit）で行う。SERIKO ループ（M-life）の毎フレーム再合成への布石＝再合成を O(elements) の転写に保つ。
3. **論拠**: 合成モデルを自前で持てば、ukadoc 準拠のピクセル忠実性・合成メソッドの追加・M2 拡張（エフェクト等）が emo 内で完結し、表示バックエンド（WUC）の制約から独立する。副次効果として、**「行列→WUC 適用方式」問題は消滅**（行列は自前合成パス内で適用）し、**AlphaMask（クリックスルーα源）は合成済み1枚から自然に生成**できる。

## Current State

- **mock-shell 実体**: `crates/areka/src/main.rs` — 単一 `BitmapSource`（base.png）＋ `HitTest::alpha_mask()` ＋ Typewriter バルーン。element 合成・surface 選択なし。
- **wintf 表示基盤（WUC 移行済）**: `BitmapSource`→WIC 読込→`Visual`→WUC `SpriteVisual` の表示経路と、per-widget `AlphaMask::is_hit`（clickthrough α源・**premultiplied BGRA**＝`from_pbgra32`）が完成済み。→ 本ユニットは「1枚物を表示する口」と「AlphaMask 供給」だけを wintf に頼る。
- **入れ子 Visual 合成**（`visual_hierarchy_sync_system`）は存在するが、**本ユニットの surface 合成には使わない**（上記方針・窓/文字レイヤ等の粗い層構成にのみ使用可）。
- **入力モデルは完成済み**: `areka_parsers::shell::parse(&str)->Shell` ✅／`balloon` ✅／`package::MountModel` ✅。**parser は転記層**＝appends/aliases/範囲の展開・実サーフェスツリー構築は本ユニット（下流）の責務（記憶 areka-parser-transcribes-tree-downstream）。
- **バルーン枠画像の読込経路は未実装**（`balloons*.png`）。

## Desired Outcome

型付き Shell/balloon モデルを入力に、自前合成コンポーネントが **surface（base＋element・入れ子参照可・配置＝行列）を1枚物ビットマップへ合成**し、シェルとバルーン枠を同一エンジンで表示できる。

**✔ 観測（単一 pass/fail・二層）**:
- (a) **合成コアの pixel 単体テスト**（オフスクリーン・表示不要）: emo2 fixture の surface0 合成結果が期待ピクセル（golden/要点サンプリング）と一致。
- (b) **専用 example 表示**: surface0＋バルーン枠の合成済みビットマップが窓に表示される（window-placement 完了を待たない＝mock-shell 級の窓を example 内で自前使用）。

## Approach

1. **合成コア（emo 所有・純粋層）**: Shell モデル → 合成プラン（layer 順・変換行列・合成メソッド）→ 合成実行 → 1枚ビットマップ。バックエンド（CPU ピクセル演算 or D2D オフスクリーン）は design で選定（下記判断材料）。
2. **アトラス管理**: 読込時に element 画像をデコード→**正規化**（キーカラー/`.pna`/self-alpha を **premultiplied BGRA へ統一**）→packing→アトラス頁へ貼付。同一 path の重複排除。
3. **配置＝行列**: element/入れ子 surface の配置を Matrix3x2 で内部表現（x,y は単位行列の特例）。適用は自前合成パス内（emo2 は平行移動のみだが構造は行列＝最初から）。
4. **入れ子サーフェス参照**: element＝{アトラス領域 | 他 surface id} の再帰合成（**循環検出必須**）。
5. **表示結線**: 合成済み1枚 → wintf の表示口（既存 `BitmapSource` 相当の供給、または合成結果を直接受ける最小 widget を新設・design 判断）＝窓あたり visual 最小限。**AlphaMask は合成済みビットマップから生成**（clickthrough 直結）。
6. **surface 指令 API＋キャッシュ**: 「surface id 切替」の命令適用口と、合成結果キャッシュ（id→bitmap・無効化規則）。M-boot は切替時のみ再合成。
7. **バルーン枠＝surface**: `balloons*.png` を同一機構で合成・表示（balloon dir は fixture パス直指定＝ベースウェアのバルーン選択は ghost 層の後続領分）。

## 設計指示・注意点（requirements/design への詳細指示）

- **合成メソッド写像表を design で作成**: ukadoc 全メソッドを「D2D 標準ブレンドで可（overlay=SourceOver・asis=α無視コピー・replace=矩形クリア＋コピー）／生ピクセル or エフェクト要（reduce=α乗算・interpolate・overlayfast の条件付き合成・blend-* 群）」に分類。**実装は emo2 使用分のみ**・表と型シームは全量（最小実装＋薄い拡張シーム）。
- **premultiplied alpha 一貫性**: wintf パイプラインは PBGRA premultiplied（`alpha_mask::from_pbgra32`）。合成演算・アトラス格納・表示供給の全段で premultiplied を貫く（straight α 混入はにじみ/暗縁の典型バグ源）。
- **透過の正規化はアトラス挿入時に一度だけ**: キーカラー（左上ピクセル）・`.pna`（黒=透明）・`seriko.use_self_alpha`（0/1/full）の解釈は**デコード→アトラス格納時**に済ませ、合成時は α 一本。emo2 fixture が使う腕を design で確認し、その腕のみ実装。`seriko.paint_transparent_region_black` の要否も同時に確認。
- **アトラス設計**: packing は最小実装で可（shelf/skyline 系）。頁サイズは 2048〜4096 推奨（D3D 上限 16384 に頼らない）・複数頁対応・**パディング 1〜2px**（転写時の bleed 防止）・element 単位の UV（領域）テーブル。
- **CPU 合成 vs D2D オフスクリーン合成の選定基準（design 判断）**: ①ピクセル忠実性（生ピクセル演算の要否）②毎フレーム再合成コスト（M-life の SERIKO ループ前提）③スレッド制約（D2D device context 単一スレッド・WUC upload は UI スレッド固定＝記憶 areka-wuc-runs-on-mta-thread）④単体テスト容易性（オフスクリーン・headless）。**合成コアの API はバックエンド非依存に切る**こと（後で差し替え可能に）。
- **再合成の予算**: M-boot＝surface 切替時のみ。M-life（seriko-loop）で毎フレーム再合成が来る前提で、アトラス転写ベースの再合成が element 数に線形・アロケーション無しで回る構造にする（合成先ビットマップの再利用）。
- **入れ子の循環検出**: element→surface 参照の再帰に訪問集合で循環を検出し、非パニックで打ち切り（parsers の寛容方針と整合）。
- **疎 id・appends/aliases の展開は本ユニット側**: parser は転記層（範囲記述子 `Vec<AppendTarget>` のまま）。実サーフェスツリーへの展開・append 適用・alias 解決は emo のツリー構築で行う。
- **DPI**: 合成はピクセル等倍（surface 原寸）で行い、拡縮は表示側（wintf/GlobalArrangement）の責務。合成パスに DPI を持ち込まない。
- **メモリ方針**: アトラス頁数と合成キャッシュの上限方針を design で明示（emo2 規模では小さいが、方針だけ最初から）。

## ukadoc 正典要点（design の前提事実）

- **element 合成**: base surface PNG に `element0..N` を **0→N 順にマージ**して一枚として扱う（＝「合成後は1枚」という正典自体が本方式と同型）。element 座標＝左上原点オフセット。
- **合成メソッド**: `overlay`（単純重ね）／`overlayfast`／`replace`／`base`／`reduce`（α乗算・RGB 無視）／`asis`（透過無視の強制上書き）／`interpolate`／`blend-*` 群。
- **透過**: 既定＝左上ピクセル＝キーカラー。`seriko.use_self_alpha,1`＝PNG α＋`.pna` 併用、`full`＝α のみ。優先: PNG α ＞ .pna ＞ キーカラー。
- **関連 descript キー**: `seriko.use_self_alpha`／`seriko.paint_transparent_region_black`／`seriko.dpi`。
- **バルーン画像**: `balloons0/1.png`（本体側）＋`balloonk*.png`（相方側）・α規則は surface と同一・最小表示は balloons0 一枚で成立。

## Scope

- **In**: 自前合成コンポーネント（合成コア・純粋層＋バックエンド）／アトラス管理（正規化・packing・UV 表）／1枚物ビットマップ生成／行列内部表現（合成パス内適用）／入れ子参照の再帰合成＋循環検出／appends・aliases・範囲の展開（ツリー構築）／表示結線（1枚供給・AlphaMask 生成）／surface 指令 API＋合成キャッシュ／バルーン枠画像（fixture 直指定）／合成 pixel 単体テスト＋専用 example。
- **Out**: **wintf Visual 入れ子合成への依存（不使用と明記）**／SERIKO アニメ再生・interval・MAYUNA（seriko-engine／seriko-loop・mayuna-compose）／テキスト描画・scroll（emo-text-layer）／collision→region 写像（collision-geometry）／バルーン所在のベースウェア解決／emo2 未使用の合成メソッド・透過腕の実装（表と型シームのみ）。

## Boundary Candidates

- **合成コア**（Shell→合成プラン→ピクセル・純粋＝オフスクリーン単体テスト可）
- **アトラス管理**（デコード・正規化・packing・UV）
- **表示結線**（1枚供給・AlphaMask・wintf 接続点＝この層だけが wintf を知る）
- 指令適用口（surface id switch）＝将来の seriko→emo channel 契約の片側

## Out of Boundary

- Window entity の生成・既定位置・ドラッグ（**window-placement**＝Window 層。本ユニットは合成と表示供給のみ）。

## Upstream / Downstream

- **Upstream**: `areka-P0-shell-parse` ✅・`-balloon-parse` ✅・`-package-mount` ✅（型付き入力）／wintf WUC 表示口・AlphaMask 基盤 ✅。
- **Downstream**: `areka-P0-emo-text-layer`（合成済み surface の上の文字層）／`areka-P0-seriko-engine`（毎フレーム再合成の駆動者＝アトラス転写が効く）／`collision-geometry`・`choice-render`・`dual-window`（増分）。

## Existing Spec Touchpoints

- **Extends**: `completed/areka-mock-shell`（表示・窓まわりの donor。合成は新規）。
- **Adjacent**: `areka-P0-window-placement`（**同じ `crates/areka/src/main.rs` 起点＝並行着手時はファイル衝突注意・順次推奨**。境界: Window entity=placement／合成＋表示供給=本ユニット）。
- 既知ドリフト解消: `balloon/model.rs:6` doc コメントの旧名 `text-layer`/`surface-engine` 参照を本ユニット着手時に追随修正（記憶 areka-engine-names）。

## Constraints

- Rust 2024・`windows` 0.62.2・tokio 禁止。**WUC/D2D を触る段は UI スレッド固定**（DispatcherQueue 親和性・MTA＋`DQTAT_COM_NONE`）。合成コアをスレッド非依存に切れるかは design 判断。
- 最小実装＋薄い拡張シーム（emo2 使用分のみ・**行列/入れ子/アトラス/自前合成の構造は最初から**）。
- 正典は ukadoc・emo2 fixture は最小適合サンプル（書式の聖典としない）。
