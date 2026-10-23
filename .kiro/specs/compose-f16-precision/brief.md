# Brief: compose-f16-precision

> **低優先度で起票**（2026-10-15 `/kiro-discovery`）。開発者の指示は「低優先度で起票」。段は据え置き・区分 D。形は決めていない。下の「推す形」と「議題」は、着手のときの要件の段で覆してよい。
>
> 出どころ（2026-10-15 開発者）: 「wintfで利用しているスワップチェーンあるいはオフラインバッファについて、u8からf16精度のバッファにして、合成精度を強化したい。」

## Problem

- 画面に出るまでの途中の入れ物が、どれも 1 色 8 ビット（0〜255 の 256 段）で持たれている。重ね合わせ・半透明・拡大縮小のたびに 256 段へ丸め直すので、丸めの誤差が段ごとに積もる。
- 特に効くのは、薄い半透明の所（α が小さいと、α を掛けた後の色は数段しか残らない＝α=10 なら 1 色あたり約 11 段）と、部品を何枚も重ねる所。これから増える描画メソッド（`draw-methods-canon`・`draw-methods-blend` の `blend-*`）やフェード（`text-reveal-fade`・`balloon-scroll-fade`）は、重ねる回数と半透明を増やす。
- 開発者は、途中の入れ物を半精度の浮動小数（1 色 16 ビットの f16）にして、合成の精度を上げたい。

## Current State（起票時の実測・main `574bf0a6`）

### 8 ビットで固定している所

- **wintf の供給面の作り手**: `crates/wintf/src/com/dxgi.rs` の `create_composition_swap_chain`。形式 `DXGI_FORMAT_B8G8R8A8_UNORM`・α は掛け済み（premultiplied）・2 枚・引数なしの固定値。説明の注釈が「emo-present 表示層の固定契約」と書いている。本番で呼ぶのはバルーンの文字の層（`crates/areka-emo-text/src/surface.rs` の `TextSurface::create`）だけ。
- **wintf の描画面（`Visual` ごと）**: `crates/wintf/src/ecs/graphics/systems/surface.rs` の `deferred_surface_creation_system` が `CreateDrawingSurface(B8G8R8A8UIntNormalized, Premultiplied)` で作る。**キャラクターのシェルとバルーンの枠はこの道**（emo-present の自前のスワップチェーンは取り除き済み）。
- **文字の層の中の入れ物**: `crates/areka-emo-text/src/surface.rs` の表と裏の 2 枚のテクスチャと読み戻し用のテクスチャ（どれも B8G8R8A8）・`crates/areka-emo-text/src/draw.rs` の `create_d2d_target_bitmap`（D2D の描き先）。
- **表示の段**: `crates/areka-emo-present/src/display.rs` が合成済みの画素を B8G8R8A8 の D2D の絵に載せ、命令の列（command list）に記録する。wintf の `render.rs` がそれを拡大率 k の変換つきで描画面へ描く。
- **画像の読み込み**: `crates/areka-emo-atlas/src/decode/wic_arm.rs` は WIC で 32bppPBGRA（8 ビット・α 掛け済み）へ変換する。16 ビットや浮動小数の元画像も 8 ビットに落とす。動く画像（`decode/image_arm.rs`）も CPU で α を掛けて 8 ビットへ。

### 一番効く所は GPU ではなく CPU の合成

- シェルとバルーンの枠の合成は **CPU の整数の計算**（`crates/areka-emo-compose/src/blit.rs`・α 掛け済みの SourceOver を u8／u32 で・`div255(v) = (v + 127) / 255`）。浮動小数を経路に入れないのは**わざと**で、答えを決定論にして、合成の檻（`golden_tests_*.rs`）が画素をバイト単位で固定するため。出力 `ComposedSurface` も u8。
- 部品を 1 枚重ねるたびに 1 回丸める。**丸めが積もるのはここ**。GPU の側で丸めるのは、拡大率 k≠1 のときの拡大縮小の 1 回と、文字の描画の 1 回くらい。
- したがって **スワップチェーンと描画面だけを f16 にしても、シェルとバルーンの絵にはほとんど効かない**（GPU に届いた時点で 8 ビットに丸め済み）。効くのは文字の層の描画と、k≠1 の拡大縮小の 1 回だけ。合成の精度を本当に上げるには、CPU の合成（u8 の入れ物）を広げるか、合成を D2D へ移すかが要る。後者は `draw-methods-canon` の未決の分かれ目「合成をどこで行うか」そのもの。

### 8 ビットを前提にしている読み手（f16 にすると直す所）

- 文字の層の読み戻し: `surface.rs` の `copy_rows`（1 行＝幅×4 バイト）・`read_back`・`begin_read_back`／`PendingReadBack::try_finish`。使い手は MCP の `dump_balloon`（`crates/areka/src/mcp/dump_balloon.rs`）。
- 当たり判定は全部 CPU の 8 ビットの α（`crates/wintf/src/ecs/widget/bitmap_source/alpha_mask.rs` の `ALPHA_THRESHOLD: u8 = 128`・emo-present の `presenter/budget.rs`・`cache.rs`）。抜き色と自前 α の判定も BGRA のバイトの一致（`normalize.rs`）。バルーンの背景色は焼いた絵の (0,0) の画素を読む（`emo2_boot/balloon_background.rs`）。
- 形式を固定で検査するテスト: wintf の `dxgi.rs` の形式の assert・`wuc.rs`・`tests/graphics/surface_pixel_equivalence_test.rs`・`tests/com/d3d11_test.rs`、emo-present の `display_gpu_tests.rs`、emo-text の読み戻しのテスト群（`draw_readback_test.rs`・`viewbox_*_tests` ほか多数）、areka の `dump_balloon_gpu_tests.rs`。先進坑 `pilot-balloon-asset-swap/capture.rs` は B8G8R8A8 以外を断る。
- MCP の `dump_surface` は CPU の `ComposedSurface` を読むので、GPU だけの変更なら影響しない。

### 確かめていないこと

- WUC／DirectComposition が f16（`R16G16B16A16_FLOAT`）の α 掛け済みの供給面を合成に受けるかは、リポジトリに記述が無い（コードにも文書にも FP16・scRGB の語は 0 件）。**起票時の見立て**: DXGI の f16 のスワップチェーンは、DWM が **線形の色空間（scRGB）** として読むのが既定。今の画素は sRGB の値（ガンマのかかった値）なので、そのまま f16 に入れると色が変わる。半透明の縁の混ざり方も、線形で混ぜるかガンマのまま混ぜるかで見た目が変わる（SSP と違って見える）。着手のとき最初に確かめる（議題 2）。
- SDR の画面では DWM の最後の出力は 8 ビット。f16 で得るのは「途中の丸めを減らす」効き目で、出口の段数は増えない。

## Desired Outcome

- 部品を何枚重ねても、半透明やフェードを重ねても、途中の丸めの誤差が目に見える縞やにじみとして積もらない。
- 精度を上げても、合成の答えが決定論で検査できるまま（今の檻の価値を失わない）。
- ukadoc の見た目（ガンマのまま混ぜる SSP と同じ混ざり方）から外れない。外れるなら areka 独自の所として対応表（`doc/COMPAT_ARCHITECTURE.md`）に記録する。

## Approach

**推す形**（起票時の見立て・要件の段で覆してよい）: 「スワップチェーンを f16 に替える」ではなく「**丸めが積もる所から順に精度を上げる**」に読み替える。

1. **先に測る**: 今の 8 ビットで、どの場面で目に見える差が出るかを測る（重ねる枚数・α の小ささ・拡大率）。差が見えなければ取り下げてよい。
2. **合成の精度は `draw-methods-canon` の分かれ目と一緒に決める**: CPU の整数の合成を広げる（u16 の入れ物・丸めを最後の 1 回だけにする）か、D2D の合成（f16 の中間の絵）へ移すか。どちらでも決定論の取り方を決め直す。
3. **wintf の入れ物の形式を引数にする**: `create_composition_swap_chain`・`CreateDrawingSurface`・文字の層のテクスチャと D2D の描き先の形式を固定値から引数へ。既定は今の B8G8R8A8 のまま。
4. **読み戻しは 8 ビットへ変換して返す**: `copy_rows` の中で f16 → u8 に変換すれば、MCP の `dump_balloon` と読み戻しのテストの多くはそのまま通る。当たり判定は CPU の 8 ビットの α のままでよい。

**採らない方向（起票時）**:

- スワップチェーンと描画面の形式だけを替えて終わりにする（CPU の合成で丸め済みなので効き目がほとんど無く、色空間の変化だけが入る）。
- HDR の表示（明るさの範囲を広げる）。今回の目的は途中の精度で、HDR 対応は別の話。
- 当たり判定を f16 の α で行う（閾値 128 の判定に精度は要らない）。

## Scope

- **In**（着手のときに切る）: 丸めの誤差の測り方と測った結果／wintf の入れ物の形式の引数化（スワップチェーン・描画面・文字の層）／色空間の扱い（線形かガンマか）の決定と実装／読み戻しの形式の変換／形式を固定で検査しているテストの更新／（`draw-methods-canon` の決定しだいで）合成の入れ物の精度。
- **Out**: HDR の表示・広い色域／画像を 16 ビットのまま読む（元の画像の精度を活かす話・別に起票）／描画メソッドの追加そのもの（`draw-methods-canon`・`draw-methods-blend`）／当たり判定の作り。

## Boundary Candidates

- 測り方（どの場面で丸めの誤差が見えるか・決定論の比較）。
- wintf の入れ物の形式（供給面の作り手・描画面・D2D の描き先）。
- 色空間（f16 を線形で持つかガンマのまま持つか・DWM への渡し方）。
- CPU の合成の精度（`areka-emo-compose`・`ComposedSurface`）と表示の段の載せ替え（`areka-emo-present` の `display.rs`）。
- 読み戻しと MCP の画像出力（`areka-emo-text` の `surface.rs`・`crates/areka/src/mcp/`）。

## Out of Boundary

- ukadoc の語の意味を変えること。
- SSP の画面の実測を写すこと（混ざり方は ukadoc と一般の α 掛け済みの合成から決める）。
- 設定画面（精度を利用者が選ぶ口）。

## Upstream / Downstream

- **Upstream**: wintf の WUC の口（`com/dxgi.rs`・`com/wuc.rs`・`ecs/graphics/systems/{surface,render}.rs`）／emo の 4 段（atlas → compose → present → text）／`draw-methods-canon` の「合成をどこで行うか」の決定。
- **Downstream**: `draw-methods-blend`（`blend-*` の重ね合わせ）・`text-reveal-fade`・`balloon-scroll-fade`・`element-animation-easing`（不透明度を時間で変える演出は丸めの段数が少ないと縞になりやすい）。

## Existing Spec Touchpoints

- **Extends**: なし（新しい spec）。持ち主の完了 spec は `areka-P0-emo-compose`・`areka-P0-emo-present`・`areka-P0-emo-text-layer`・`areka-P0-present-gpu-transform-scale`・`wintf-dcomp-to-wuc-migration` ほか（`.kiro/specs/completed/`）。
- **Adjacent**:
  - `draw-methods-canon` — 「合成をどこで行うか（CPU か D2D か）」と決定論の取り方を未決のまま持つ。本 spec の合成の精度の形はこの決定に従う＝**先に `draw-methods-canon` が決める**。同じ時期に着手するなら一緒に決める。
  - `draw-methods-blend` — `blend-*` の計算の精度。
  - `text-reveal-fade`・`balloon-scroll-fade` — 文字の層の α を時間で変える。文字の層の形式を替えるならぶつかる。
  - `mcp-receiver-bugs` — MCP の画像出力の周り（`dump_balloon`）を触る。

## Constraints

- **決定論のテストは必達**: 精度を上げても、合成の答えを決定論で検査できる形を残す。GPU の f16 の答えは機種で揺れうるので、比較の許容差の決め方が要る。
- **GPU のテストは実 GPU**（WARP などへ替える案は出さない）。
- **正典の見た目**: 書き手が見る色が変わらない（色空間の取り違えで暗く・白っぽくならない）。
- **メモリ**: f16 は 1 画素 8 バイトで今の倍。キャラクターとバルーンの窓の数だけ倍になる。
- **α 付き画像が原則**: 抜き色・自前 α の判定（8 ビットの一致）はそのまま。

## 議題（着手のときに開発者へ・答えで作業が変わるもの）

1. **目的の読み替え**: 開発者の言う「スワップチェーンあるいはオフラインバッファ」は GPU の入れ物だが、丸めが積もるのは CPU の合成。GPU の入れ物だけを替える（効き目は文字の層と拡大縮小だけ）か、CPU の合成の精度まで含める（`draw-methods-canon` と組む）か。
2. **色空間**: f16 を線形（scRGB）で持って線形で混ぜる（半透明の縁が明るくなり、SSP と見た目が変わる）か、ガンマのままの値を f16 に入れて今と同じ混ぜ方を保つ（DWM へ渡すときの扱いを確かめる）か。
3. **先に測るか**: 着手の最初に「8 ビットで目に見える差が出る場面」を測り、差が見えなければ取り下げるか。
4. **既定をどちらにするか**: f16 を常に使うか、既定は 8 ビットのままで条件（重ねる枚数・フェード中）で切り替えるか（メモリが倍になるため）。

## 2026-10-15 `/kiro-discovery` の続き（合成は D2D の道）

- 同じ日の続きで、開発者が「D2D合成アルゴリズムの改善も入れる予定」と決め、`draw-methods-canon` の議題 1 は Direct2D の道になった。wintf の合成の規則の口と自前の合成の入れ物は `wintf-d2d-composite-rules` が作る。
- **上の見立ての読み替え**: 丸めが積もる CPU の整数の合成は、D2D の命令の列へ移る見込み。そうなれば f16 は「スワップチェーンを替える」でなく、**D2D の途中の入れ物の精度の設定**（`ID2D1DeviceContext::SetRenderingControls` の buffer precision を 16 ビットの浮動小数に）と、自前の合成の入れ物の形式で得られる（起票時の見立て）。描画面（B8G8R8A8）とスワップチェーンはそのままでよい。
- **色空間の見立て**: D2D は sRGB の印の無い形式（`B8G8R8A8_UNORM`）の値を変換せずに混ぜるので、途中を f16 にしてもガンマのまま混ぜる今の混ざり方を保てる見込み（上の議題 2 の答えの候補・着手のとき確かめる）。
- **前提の追加**: `wintf-d2d-composite-rules`（自前の合成の入れ物）。議題 1（目的の読み替え）は「合成の精度まで含める」に寄った＝D2D の道なら同じ口で済む。
