# Brief: areka-P0-emo-present

> **種別**: 本坑（main）。⑥ emo トラック直列チェーン **3/3**（emo-atlas → emo-compose → **emo-present**。旧 `areka-P0-emo-surface` の 2026-07-03 粒度分割＝本ユニットで旧ゴール完走）。
> **方針正本**: 合成は emo 自前・アトラス転写・1枚物（記憶 areka-emo-own-compositor-atlas／roadmap emo 節）。

## Problem

合成コア（emo-compose）が生む 1枚物ビットマップは純粋データであり、**画面に出す結線——wintf への供給・クリックスルー用 AlphaMask の生成・surface 切替の指令口・合成キャッシュ——が存在しない**。ここが埋まって初めて旧 emo-surface のゴール（surface0＋バルーン枠の表示）が完走する。

## Current State

- **emo-compose（直列2・前提）**: `compose(surface_id) -> 合成済み premultiplied BGRA 1枚`（純粋）。
- **wintf 表示基盤 ✅**: `BitmapSource`→`Visual`→WUC `SpriteVisual` の表示経路。per-widget `AlphaMask::is_hit`（**premultiplied BGRA**・`from_pbgra32`）が clickthrough の α源（`WS_EX_TRANSPARENT` 動的トグル・07-02 完了）。
- mock-shell（`crates/areka/src/main.rs`）に窓生成・クリックスルー登録の実績コードあり（example の donor）。

## Desired Outcome

合成済み1枚を wintf の窓に表示し（**窓あたり visual 最小限**・入れ子 Visual 合成不使用）、**AlphaMask を合成結果から生成してキャラ領域のみクリック可**とし、surface id 切替の指令 API と合成キャッシュを備える。

**✔ 観測（単一 pass/fail）**: **専用 example** が emo2 fixture から surface0＋**バルーン枠（`balloons*.png`）**を表示し、(a) 見た目が emo-compose の golden と一致（b) キャラ不透明領域のみクリック捕捉・透明域は背後へ透過（clickthrough 実挙動）（c) 指令 API で surface id を切り替えると表示が更新される。window-placement 完了は待たない（mock-shell 級の窓を example 内で自前使用）。

## Approach

1. **表示口**: 合成済み BGRA バッファ→WUC surface 更新の最小 widget（既存 `BitmapSource` の「ファイルから」を「メモリから」に置き換えた供給路。既存流用 or 最小新設は design 判断）。**窓＝visual 1〜数枚**（surface 本体＋将来の text-layer 用の口だけ・粗い層構成のみ許容）。
2. **AlphaMask 生成**: 合成結果（premultiplied）から `from_pbgra32` 相当で AlphaMask を構築し hit-test へ供給——clickthrough 直結。**surface 切替時に AlphaMask も同期更新**（ズレ＝クリック領域の食い違いバグ源）。
3. **指令 API**: `show_surface(scope, surface_id)` 級の適用口（＝将来の seriko→emo channel 契約の片側。M-boot は直接呼出で可・channel 化は kanade/seriko 結線時）。
4. **合成キャッシュ**: surface id→合成済みバッファ（LRU or 全保持・emo2 規模では全保持で可）＋無効化規則（アトラス再構築時）。emo-compose は純粋関数のまま・キャッシュは本層が持つ。
5. **バルーン枠**: `balloons0.png` 等を balloon dir（**fixture 直指定**・ベースウェアのバルーン選択は ghost 層の後続領分）から同一機構で合成・表示。
6. **更新スレッド規律**: WUC surface 更新・visual 操作は **UI スレッド固定**（DispatcherQueue 親和性）。合成（CPU）を worker で行う場合はバッファを channel/queue で UI スレッドへ渡す（並行モデル正本: render は UI スレッド・他 actor は channel で送る）。

## クロスユニット契約（後続を詰ませない事前考慮・2026-07-03）

- **text-layer スロットの予約（詰み防止）**: 窓の visual 構成に**文字層の口を最初から予約**する（surface visual の上の独立レイヤ）。M1 の emo-text-layer は独立レイヤ描画（typewriter の毎グリフ更新が surface 再合成を強要しない）・M2 のポップアート装飾では合成パス内レイヤ化の再設計余地——この**二者を吸収できる seam**（text 層の差し込み点）を design で確認。予約しないと emo-text-layer 着手時に visual 構成の作り直しになる。
- **bind 有効集合の初期解決**: emo2 の surface1000 表示には bindgroup default（`bindgroupN.default`・MAYUNA descript）の解決が必要。指令 API は surface id に加え **bind 集合を運べる形**にする（emo-compose の `compose(surface_id, active_binds)` 契約の呼び手側・将来の seriko→emo channel 契約の片側）。M-boot は descript default から静的解決。
- **window-placement との統合 seam**: 本ユニットの example は仮設窓だが、表示装着 API は「**Window entity（handle）を受け取って surface を載せる**」形に切る——window-placement が生成する窓へ M-boot 統合でそのまま装着できる契約（どちらが先に完了しても結線可能）。
- **ulw-removal との API 変動調整**: `wintf-ulw-removal` は `CompositionMode` collapse＝areka 側呼び出し（`CompositionMode::DComp` 指定箇所）を壊す破壊的変更。**順序調整**（ulw-removal 先行が理想）または本ユニット側の追随を織り込む（並行時は rebase 責務を明確に）。
- **通信モデル（actor-foundation との契約）**: 指令 API（`show_surface` 級）は将来 `areka-P0-actor-foundation` の envelope 規約に載り、**UI 配送ブリッジ**（worker→UI pump への queue＋wakeup・foundation が提供）経由で届く——M-boot は直接呼出で開始するが、**指令 API のシグネチャは「メッセージ enum の1バリアントに転写できる形」**（`Send` な所有データ・借用なし・応答不要 or 返信 Sender 同梱）に最初から切ること。channel 化時に API 再設計が要らないことが受け入れ基準。actor-foundation とは**並走可**（本ユニットは直接呼出で完結・結線は kanade/seriko 時）。

## 設計指示・注意点

- **AlphaMask と表示の原子性**: 表示バッファと AlphaMask は**同じ合成結果**から作り、切替は対で入れ替える（片方だけ古い状態を作らない）。
- **実 DPI での観測（2026-07-05 追加・window-placement リジェクト教訓）**: 表示と AlphaMask クリック判定の観測は**実 DPI（dpi≠96）でも実施**する。surface 等倍表示の DPI スケール方針（合成＝物理 px 等倍・表示側の論理/物理変換の帰属）を design で確定——wintf の座標契約（`Monitor.work_area`/`WindowPos`=物理・`BoxStyle`=論理・記憶 areka-window-placement-dpi-coordinate-defect）と同じ整理に乗せ、**下流 window-placement が同契約をそのまま前提にできる形**で文書化する。dpi=96 のみの確認は不十分。
- **premultiplied のまま WUC へ**: WUC surface のピクセル形式（BGRA premultiplied）と合成出力を一致させ、途中変換を挟まない。
- **サイズ変化**: surface ごとに原寸が違い得る——窓/visual サイズの追随規則（原寸表示・DPI 拡縮は wintf 側）を design で明確化。
- **キャッシュ無効化**: M-boot ではアトラス不変＝実質不要だが、無効化の口だけ設ける（ghost 再読込・将来の動的差替えに備えた**構造**）。
- **example の位置づけ**: 観測用の専用 example（`crates/areka/examples/` 等）とし、`main.rs` の書換えは最小限に留める——**window-placement と同じ `crates/areka` を触るため、同時着手しない**（順次推奨・roadmap 記載）。

## ukadoc 必読（design 着手時に ukadoc MCP `get_doc`/`search_docs` で正典参照・2026-07-03 総ざらい）

- **必読**: `descript_balloon` **全文**（バルーン枠描画に効くキーの全容——`use_self_alpha`/`paint_transparent_region_black`〔shell と別定義〕・`overlay_outside_balloon`〔marker/online 表示のクリップ規則〕・有効描画領域/テキスト領域系キー）。配置系は `descript_shell` の **`sakura.balloon.offsetx/offsety`**（バルーンアンカー）・`sakura.balloon.alignment`、`descript_shell_surfaces` の **`balloon.offsetx/offsety`**（surface 別上書き）、`descript_ghost` の **`sakura.balloon.defaultsurface`**（既定バルーン面番号）。
- **brief 未網羅→design で埋める項目**: ① バルーン枠の**有効描画領域**（validwidth/validrect 系・テキストが載る領域とクリップ——ukadoc の該当キーを design で確定。**emo-text-layer が同じ領域定義を消費**するため公開形に）② `balloons*.png`/`balloonc*.png`/`arrow0/1.png`/`marker.png` の**役割分担表**（M-boot は枠のみ・arrow/marker/onlien は後続と明記）③ balloon `.pna`/α 規則（fixture 実測: kakukaku は png のみ）④ `sakura.balloon.alignment` の値域（fixture: left/right——複合値の有無を確認）⑤ `seriko.zorder`/`seriko.sticky-window`（emo2 未使用ならシームのみ・確認）。
- **具体指示**: design 冒頭で `get_doc('descript_balloon')` を読み、**「枠描画に効くキー」「テキスト領域に効くキー（→emo-text-layer へ引き継ぐ）」「M1 対象外」の3分類表**を design.md に載せること。バルーンアンカー（offsetx/y）は window-placement のバルーン追従 offset と同じ座標系か照合。

## Scope

- **In**: メモリ供給の表示口（最小 widget/流用）／AlphaMask 生成＋同期更新／指令 API（id 切替）／合成キャッシュ＋無効化口／バルーン枠表示（fixture 直指定）／専用 example／clickthrough 実挙動の確認。
- **Out**: 合成そのもの（**emo-compose**）／アトラス（**emo-atlas**）／窓の既定位置・ドラッグ機構化（**window-placement**）／テキスト描画（**emo-text-layer**）／SERIKO 再生（seriko）／channel 契約の確定（kanade/seriko 結線時）。

## Boundary Candidates

- 表示口（バッファ→WUC・wintf を知る唯一の層）／キャッシュ＋指令 API（emo のランタイム状態）／AlphaMask 供給（hit-test 接続点）。

## Out of Boundary

- Window entity の生成・配置・ドラッグ（window-placement）。example 内の窓は観測用の仮設。

## Upstream / Downstream

- **Upstream**: `areka-P0-emo-compose`（直列2）→ その先に `emo-atlas`（直列1）／wintf 表示・AlphaMask・clickthrough 基盤 ✅。
- **Downstream**: `areka-P0-emo-text-layer`（表示済み surface の上の文字層＝本ユニットの表示口を前提）／`areka-P0-seriko-engine`（指令 API の将来の呼び手）／`collision-geometry`・`choice-render`・`dual-window`（増分）。

## Existing Spec Touchpoints

- **Extends**: `completed/areka-mock-shell`（窓・clickthrough 登録の donor）。
- **Adjacent**: `areka-P0-window-placement`（**同じ `crates/areka` 起点＝並行着手はファイル衝突注意・順次推奨**。境界: Window entity=placement／表示供給＋emo ランタイム=本ユニット）。

## Constraints

- Rust 2024・`windows` 0.62.2・tokio 禁止。**WUC 更新は UI スレッド固定**（MTA＋`DQTAT_COM_NONE`＝記憶 areka-wuc-runs-on-mta-thread）。
- 最小実装＋薄い拡張シーム（キャッシュ無効化・channel 化の**口**だけ最初から）。
- 正典は ukadoc・emo2 fixture は最小適合サンプル。
