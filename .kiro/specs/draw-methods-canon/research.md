# ギャップ分析: draw-methods-canon

> 2026-11-27・`/kiro-validate-gap`（`/kiro-start` の中で実行）。基準は main `af6b9001`（ワークツリーの枝 `claude/kiro-start-draw-methods-canon-bee12e`）。要件は `requirements.md`（未承認の下書き）。行番号はこの時点の木。

## 1. 分析の要約

- **読み手と「描けるか」の門は、型の口がもう揃っている**。`ComposeMethod` は全部の語を持ち（`crates/areka-emo-compose/src/method.rs:30-56`）、門 9 か所はどれも `is_implemented`／`is_implemented_name`（同 133-135・182-184）の 1 関数を通る。読み手の `Element` に欄が無い（`crates/areka-parsers/src/shell/model.rs:74-83`）ことと、画像の語が `overlay`・`base` だけ（`decode.rs:211-213`）なことが Requirement 1 の穴。警告の洪水は既に直っている（`plan.rs:607` は `debug!`・読み込みの 1 回は `plan_issues.rs:75` の `ShapeIssue::UndrawnPattern`）。
- **合成の答えは今、CPU の 1 枚の絵が唯一の出どころ**で、そこから 5 つの読み手が派生している（表示の記録 `display.rs:158-235`・α マスク `show.rs:231-237`・名前の解決 `hit.rs:120`・書き出し `snapshot.rs:47/72/93` と `read.rs:242-269`・窓を出さない書き出し `offline_dump/render.rs:76`）。合成を GPU へ移すと、5 つが同時に出どころを失う＝**Requirement 7 は画面の移しと切り離せない**（切り離すと CPU の合成が残る）。
- **wintf の口のうち、areka から使えるのは「節の重ね方」「1 画素と全体の読み戻し」まで**（`draw_content.rs:8-39`・`readback.rs:53-83`）。クリックの透過（`hit_test/mod.rs:213-247`）は CPU の `AlphaMaskResource` しか読まず、合成の入れ物を読む当たりの形が無い。描画の木を画面の外で描く手順（`emit`・`compose_planned`）と WARP の足場（`emit_test_support.rs:60`）は `pub(crate)`。**開発者の決め（2026-10-15 ⑶「クリックの透過の α は入れ物から 1 画素をその場で読み戻す」）を満たす口は wintf に無い**。
- **`asis` は画像の読み手（emo-atlas）の正規化が答えを決める**。透過色を `0,0,0,0` へ抜き（`normalize.rs:152-168`）、α＝0 の縁をトリムで切る（`trim.rs:1-7`）ので、正典の「透過色がそのまま表示」は今の頁の画素から復元できない。乗算しない読み `decode_unmultiplied`（`decode.rs:179-184`）は既にあり、`hit_mask.rs:72-98` が前例。
- **研究の要る所**: SSP の MCP で測る 2 点（仮定 6・8）、D2D の命令の列の中の重ね方が何に効くか、描画の木と画面の外の再生の一致、範囲だけの描き直しの効き（今の wintf の側は毎回全部＝`damage.rs:106-107` の `RootList`）。

## 2. 今の作り（調べた事実）

### 2.1 読み手（`crates/areka-parsers/src/shell/`）

| 事実 | 根拠 |
|---|---|
| `Element` は `layer`・`path`・`x`・`y` だけ。描画メソッドもオプションの並びも持たない | `model.rs:74-83` |
| 画像の element定義の語は `overlay`・`base` の完全一致だけ | `decode.rs:206-213` |
| 箱の語は `balloon` だけ。描けない行の転記は、2 つの述語がどちらも偽の行を拾う（3 つの転記の分け方が 1 か所に連動している） | `decode.rs:215-219`・`undrawn.rs:9,51` |
| pattern定義の語は `DrawMethod`（原文の文字列）で運ばれ、比べ方は正規化（大小文字・`-`・`_` を無視） | `model.rs:175-187`・`pattern_word.rs:32-52` |
| `blend-*` の綴りの一覧は parsers に無い（`parse_blend` は compose の私有） | `method.rs:196-` |

### 2.2 合成の段（`crates/areka-emo-compose/`）

| 事実 | 根拠 |
|---|---|
| 「描けるか」は `Overlay`（`overlay`・`add`・`bind`）だけ真 | `method.rs:133-135`・`176`・`182-184` |
| 門 9 か所＝`plan.rs:601`・`nesting.rs:196`・`nesting.rs:308`・`plan_always.rs:76`・`blit.rs:121`・`blit.rs:250`・`bind_steps.rs:60`・`plan_bounds.rs:154`・`plan_issues.rs:75`（brief の測りと一致。`blit.rs` は 681 行に減っている） | grep `is_implemented` |
| 命令 `BlitOp` は「要素・平行移動・語」の**平らな列**。入れ子の面（element定義の子・コマの先）は畳み込まれて 1 列になる | `plan.rs:59-66` |
| 合成は premultiplied SourceOver の CPU の整数の転写。式は `overlay_op` の 1 か所 | `blit.rs:146-199`・`228-261` |
| 変わった矩形だけ CPU で重ね直す（下敷き形）| `blit_rect.rs`・`lib.rs:202`・`show.rs:124-158` |
| 合成のたびの記録は `debug!`、読み込みの 1 回だけ `warn!`（申し送りの洪水は既に止まっている） | `plan.rs:607`・`plan_issues.rs:60-95` |
| 当たり判定の領域は面の番号だけで引く（`HitRegions` が在ればそれ・無ければ `collisions`） | `world.rs:167-175` |
| pattern の `base` のコマは描かない（`table.rs:850-854` がそれを固定） | `crates/areka-seriko/src/table.rs:850-854` |
| 見本 `golden_tests*.rs` は 8 ファイル（支え 1 を含む） | `ls golden_tests*.rs` |

### 2.3 画像の読み手（`crates/areka-emo-atlas/`）

| 事実 | 根拠 |
|---|---|
| 読みは WIC の PBGRA（乗算済み）。乗算しない読み `decode_unmultiplied`（抜き色・`use_self_alpha`・`.pna` を当てない）が別に在る | `decode/wic_arm.rs:139-145`・`decode.rs:179-184` |
| 正規化は「α を 255 にするか」と「抜き色」の 2 つ。抜き色の画素は `0,0,0,0` | `normalize.rs:37-44`・`67-82`・`152-168` |
| α＝0 の縁はトリムで切る。原寸は `AtlasEntry.original` に残る | `trim.rs:1-22`・`table.rs:58-74` |
| `.pna` は有無を数えるだけで、画素には当てていない | `decode.rs:146-147`・`wic_arm.rs:147-150` |
| `UseSelfAlpha::Off` は PBGRA で読んだ色に α＝255 を当てる（半透明の画素は暗く出る見込み。範囲外の既存の疑い・**要確認**） | `normalize.rs:76` |

### 2.4 表示の段（`crates/areka-emo-present/`）

| 事実 | 根拠 |
|---|---|
| 合成した原寸の CPU の絵を毎回 `CreateBitmap` で上げ、1 本の閉じた命令の列（根の命令の列）にする | `display.rs:158-235`・`show.rs:171-211` |
| 描画面の持ち主は `emo-surface` の 1 実体で、子の節を持たない（`GraphicsCommandList`＋`HitTest::alpha_mask`＋`AlphaMaskResource`） | `mount.rs:183-193` |
| α マスクは原寸の CPU の画素から 1 適用に 1 回作り、`AlphaMaskResource` と `shown_mask` に同じものを渡す | `show.rs:231-246`・`372-380`・`budget.rs` |
| 名前の解決は `shown_mask` で α を確かめてから GPU の ID の描き先を 1 点読む | `hit.rs:111-125` |
| 書き出しの口は **`presenter/snapshot.rs`**（`last_shown`・`compose_alone`・`compose_last_shown`）で、`ComposedSurface`（CPU の型）を返す。`read.rs` の `read_back` は `RuntimeWiring` 経由の別の口 | `snapshot.rs:47,72-93`・`read.rs:242-269`・`crates/areka-runtime/src/frame/wiring.rs:279` |
| MCP の入口は `ComposedSurface` の `bytes()`・`width()`・`height()` を読むだけ（型が残れば入口は変えずに済む） | `crates/areka/src/mcp/dump_surface.rs:300-323`・`dump_balloon.rs:55-58` |
| GPU のテストは実 GPU の `GraphicsCore::new()` で作る | `display_gpu_tests.rs:38`・`presenter/hit_picture_tests.rs:141` |

### 2.5 wintf の口（触らない約束の中で使えるもの）

| 口 | 公開 | 根拠 |
|---|---|---|
| 節の混ぜ方 `MixMode::{Composite(D2D1_COMPOSITE_MODE), Blend(..)}` と `VisualDrawContent::new_ex`／`new_blend` | `pub` | `draw_content.rs:8-104` |
| 非 `SOURCE_OVER` の重ね方は絵を切り抜きまで透明で広げて効く（`DESTINATION_IN` は絵の外にも効く） | 文書 | `draw_content.rs:25-31` |
| 不透明度の層は透明から始まる（中の混ぜ方は層の中で先に描いた画素に効く） | 文書 | `draw/README.md:42` |
| `SurfaceReadback::read_pixel`／`read_all`（**描画面の物理ピクセル**・最後に合成し終えた入れ物） | `pub` | `readback.rs:40-83` |
| 1 画素の待ち＝中央値 516 µs・95% 点 747 µs（実 GPU）。全体 512×512＝中央値 1.88 ms | 記録 | `.kiro/specs/completed/wintf-d2d-composite-rules/design.md:689` |
| クリックの透過は `hit_test_in_window` → `alpha_mask_hit` → `AlphaMaskResource`（CPU の 1 ビットの表）。入れ物を読む当たりの形は無い | — | `clickthrough/controller.rs:161`・`hit_test/mod.rs:213-247`・`356-362` |
| 描画の木を画面の外の絵へ描く手順（`emit`・`compose_planned`・`SurfaceComposite`） | `pub(crate)` | `composite.rs:59-146`・`visual/draw/emit.rs` |
| WARP の装置（`warp_device`・`warp_dc`） | `pub(crate)` | `emit_test_support.rs:60-95` |
| D3D11 の装置を作る下請け `wintf::com::d3d11::d3d11_create_device`（WARP も指せる） | `pub` | `crates/wintf/src/com/d3d11.rs:8`・`lib.rs:2` |
| 範囲だけの描き直しの計画: 根の命令の列が替われば全部（`RootList`）・変わった件の範囲が全面なら全部（`CoversSurface`）・描く矩形は交わる件で閉じる（全部の件で・変形の有無に依らない）・非 `SOURCE_OVER` の件の範囲は切り抜きか全面 | — | `damage.rs:102-231`・`draw/README.md:7-15` |

### 2.6 本体と実行系

- 窓を出さない書き出しは `Composer::compose`（CPU）を直に呼ぶ（`crates/areka/src/offline_dump/render.rs:64-80`・`mod.rs:268`）。`structure.md:288` は `offline_dump/` に `wintf` を直に使わせない（`areka-emo-present` の `load_shell_target`・`ShellTarget` は使ってよい）。
- バルーンの背景色はバルーンの**アトラスの頁**の原点画素を読み、合成の答えを読まない（`crates/areka-runtime/src/balloon_background.rs:1-40`）。アトラスは CPU の焼きのまま残るので、**仮定 10 は「付け替え不要」**。

## 3. 要件 → 今の資産の対応（ギャップの印: Missing／Unknown／Constraint）

| 要件 | 今の資産 | ギャップ |
|---|---|---|
| 1.1–1.2 語を運ぶ・完全一致 | `is_image_element_method`・`DrawMethod` | **Missing**: `Element` の欄。**Constraint**: `blend-*` の完全一致の綴りの一覧が parsers に無い（compose の私有）＝語の一覧を 2 か所に持たない形が要る（議題 11） |
| 1.3–1.5 3 つの転記の分け方 | `undrawn.rs:51` の連動 | 述語を広げれば自動で連動（今の形を保てる） |
| 2.1–2.6 6 つの意味 | `MixMode::Composite` | `overlay`＝`SOURCE_OVER`・`overlay-fast`＝`SOURCE_ATOP`・`interpolate`＝`DESTINATION_OVER`・`reduce`＝`DESTINATION_IN` は D2D の式が正典の文と一致。**Constraint**: `replace` は範囲がトリム前の原寸（`BOUNDED_SOURCE_COPY` は絵の矩形＝トリム後でしか効かない）。`asis` は §4.1 |
| 2.7 入れ子の面の `asis` | — | **Missing**: 入れ子の面を 1 枚の絵として扱う単位（今の命令は平ら・`plan.rs:59-66`）。乗算済みの絵の α を 1 にすると「黒の上に重ねた」答えと同じ（黒の矩形＋`SOURCE_OVER` で描ける） |
| 3.1–3.4 ベースレイヤ | 描く順（`plan.rs`） | 1 枚の描画面の上で順に重ねれば、element定義・着せ替え・コマとも「先に描いたもの」がベースになる。**Unknown**: 複数のアニメーションの先に描いたコマを含めるか（仮定 6）・空のベースの語（仮定 8）＝SSP で測る |
| 3.5 はみ出しと外形 | `plan_extent.rs` | 正典は「SSP はベースレイヤに全透明領域が補われる」（`introduction_drawmethod`）。外形は持ち分の外のまま |
| 4.1–4.7 pattern の `base` | 門 9 か所 | **Missing**: 下の面の描画を落として先の面に替える段（`push_direct` の分岐・`plan.rs:600-615`） |
| 4.5–4.6 当たり判定の切り替え | `hit_regions(id)` | **Missing**: 今のコマの `base` を知る当たりの領域の引き方（`world.rs:167-175`・`hit.rs:86-88`）と、ID の描き先の塗り直しの鍵（`hit.rs:121-124` は面の番号だけで照らす） |
| 6.1–6.4 記録 | `ShapeIssue::UndrawnPattern` | **Missing**: element定義の `blend-*` の行の記録の出どころ。読み手が運ぶと `undrawn.rs` から外れるので、合成の 1 か所（`plan_issues.rs`）が element定義の行も数える形が要る |
| 7.1 クリックの透過 | `AlphaMaskResource` | **Constraint**: 開発者の決め ⑶（1 画素をその場で）は wintf の当たりの形の追加が要る。wintf に触らずに済む道は「全体の読み戻し → CPU の 1 ビットの表」だけ（⑶ から外れる） |
| 7.2 名前の解決 | `shown_mask`＋ID の描き先 | `SurfaceReadback::read_pixel` は areka から使える。ただし 7.1 と同じ出どころへ一緒に付け替える（`hit-id-pass` の申し送り） |
| 7.3 `dump_surface` | `snapshot.rs` | **Constraint**: 答えは**原寸**（k に依らない・`read.rs:236-241`）。画面の入れ物は物理ピクセル（k 倍）＝そのままでは形が変わる。原寸で画面の外に描いて全体を読む道が要る。触るのは `read.rs` でなく `snapshot.rs`（ウェーブの約束の文は直す） |
| 7.4 窓を出さない書き出し | `offline_dump/render.rs` | **Constraint**: `wintf` を直に使えない。emo-present の口を通す（§4.3） |
| 7.5 バルーンの背景色 | アトラスの頁 | ギャップ無し（付け替え不要） |
| 7.6 読めないとき | `PixelRead::Unreadable`・`SurfaceImage::Unreadable` | 型は揃っている |
| 8.1 `overlay` だけの見え方 | CPU の見本 | **Unknown**: CPU の `div255` と D2D の丸めの差（±1 の見込み・WARP で測る） |
| 8.2 範囲だけの描き直し | CPU の `blit_rect.rs` | **事実の訂正**: 今も wintf の側は毎回全部を描く（根の命令の列が替わる＝`RootList`）。範囲だけなのは CPU の重ね直しだけで、合成を GPU へ移すとこの効きの置き場ごと無くなる。節の形でも、まばたきは下地の大きな絵と交わって `CoversSurface`（`damage.rs:212-230`）＝**今と同じく全部**（仮定 11） |
| 8.3 同じフレーム | `apply_show` の単一漏斗 | 節・命令の列はどちらも同じ回の `render_surface` で描かれる。読み戻しに頼る読み手は `render_surface` の後に読む（同じ刻みの中・**順序の口は要確認**） |
| 9.2 WARP のテスト | wintf の足場（私有） | **Constraint**: §4.4 |
| 10.x 台帳・§8・実機 | `assets.toml`・`COMPAT_ARCHITECTURE.md` §8 | 行を直すだけ |

## 4. 仮定ごとの詳しい分析

### 4.1 仮定 4: `asis` の透過色と `normalize.rs`

- 正典: 新規レイヤの透過（透過色・`.pna`・`use_self_alpha` の α）を無視して重ねる。最初の element でも「透過色がそのまま表示」は有効（`introduction_drawmethod`）。
- 今の頁の画素では復元できない理由:
  1. 抜き色の画素は `0,0,0,0`（`normalize.rs:154-168`）。抜き色だけの絵なら元の色＝抜き色なので理屈では戻せるが、
  2. α を持つ絵は PBGRA で読む（`wic_arm.rs:139-141`）＝α＝0 の画素の色は消え、α が小さい画素は色の精度が落ちる。
  3. α＝0 の縁はトリムで切られる（`trim.rs`）。全部が透明な絵は `placement: None` で頁に載らない＝`asis` で出すべき色が丸ごと無い。
- 道:
  - **A. アトラスに「`asis` の読み」の絵を焼く**（`decode_unmultiplied` で読み、α＝255・抜き色なし・トリムなし）。`asis` で参照される絵だけ。触るのは emo-atlas の `manifest.rs`（語を読んで要る絵を数える・pattern の語は既に読んでいる＝`manifest.rs:89`）・`normalize.rs`（扱いの 1 種）・`bake.rs`・`table.rs`（鍵に扱いを足す）・`trim.rs`（飛ばす分岐）。
  - **B. 合成の側に別の表を持つ**（`RegionMasks` の前例＝`hit_mask.rs:63-98`・`shell_target.rs:570` のように読み込みで 1 度 `decode_unmultiplied`）。emo-atlas の本体に触らない。描く絵の出どころがアトラスと別表の 2 か所になる。
  - C. 近似（乗算済みから割り戻す・α＝0 は黒）。正典から外れる。
- **ウェーブの触る表との関係**: emo-atlas はウェーブ C28 の触る表に無い。C28 のほかの 5 本（`sylphya-sqlite-kvs`・`talk-script-metadata`・`balloon-script-offset-align`・`talk-thread-count-growth`・`seriko-talk-clock-fidelity`）はどれも emo-atlas に触らない＝**広げても重なりは 0**。ウェーブの外では `animated-image-import`（まだ着手前）が `manifest.rs` を触るので、列の順（本 spec が先）を守れば足りる。
- 推奨は §6 の議題 2。

### 4.2 仮定 9: 窓を出さない書き出しと wintf の禁止

- 制約: `offline_dump/` は `wintf` を直に使わない（`structure.md:288`）。`areka-emo-present` は使ってよい。emo-present は wintf に依存してよい（`Cargo.toml`）＝**emo-present の口を通せば構造の規則は破らない**。
- 問題は「画面と同じ規則で描く」の担保。画面の答えは wintf の `render_surface`（描画の木なら `emit`）が出す。その手順は `pub(crate)`（`composite.rs:80-146`）。
- 道:
  - **A. wintf の別の spec で「描画の木（または命令の列）を画面の外の絵へ描く口」を公開する**。offline・`dump_surface` の原寸・WARP のテストが wintf の同じ手順を通る＝実現が 1 つ。wintf の変更なので本ウェーブの外。
  - **B. emo-present が同じ記録を自分で画面の外へ再生する**（節ごとの命令の列と重ね方を、順に `DrawImage(命令の列, 重ね方)` で描く）。wintf の文書の規則（`draw_content.rs:25-31`・README の層の旗）を写すことになり、実現が 2 つ。一致は実 GPU のテスト（窓も画面も使わない普通のテスト）で wintf の読み戻しと比べて固める。
  - **C. 画面も 1 本の命令の列にする**（§4.6 の B 案）。画面（根の命令の列）と画面の外が同じ列を再生する＝実現が 1 つで wintf を触らない。ただし節ごとの重ね方（`MixMode`）を使わない。
- 装置: offline は窓を作らないだけで、D3D11 の装置は作れる。hardware か WARP かは設計で決める（答えを画面と同じにするなら hardware）。

### 4.3 仮定 11: 範囲だけの描き直しと「画素どおり」の申告

- **今の効きの正体**: CPU の `blit_rect.rs` が変わった矩形だけ重ね直すが、その後に原寸の絵を丸ごと `CreateBitmap` で上げ直し（`show.rs:198-201`）、根の命令の列を差し替える（`mount.rs:227-240`）ので、wintf は毎回 `FullReason::RootList` で全部を描く（`damage.rs:106-107`）。GPU の側は今も全部。
- 合成を GPU へ移すと CPU の重ね直しは置き場ごと消える。節の形にしても:
  - 描く矩形を交わる件で閉じる規則は変形の有無に依らず全部の件に掛かる（`damage.rs:209-231`）＝まばたきの部品が下地の大きな絵と交われば `CoversSurface`。
  - `SOURCE_OVER` でない件の範囲は切り抜きか全面（`draw/README.md:7`）＝新しい 5 語を 1 つでも含む面は全部。
- 「画素どおり」の申告を読むのは wintf の計画（`damage.rs` の `plan`）だけで、申告が正しいかは D2D の振る舞い（WARP で測った R1）に依る＝**仕組みの持ち主は wintf**（この spec は申告する側）。等倍でない k（DPI 125% など）では絵を変形して描くので、申告できるのは k が整数のときに限られる見込み。
- 要件 8.2 の今の文（「変わった範囲だけを描き直す」）は、GPU の側では今も満たしていない。

### 4.4 仮定 13: WARP のテストの置き場

- `warp_device` は `pub(crate)`（`emit_test_support.rs:60`）。ただし **WARP の装置そのものは emo-present のテストが自分で作れる**: `wintf::com::d3d11::d3d11_create_device`（`pub`・`D3D_DRIVER_TYPE_WARP` を渡せる）と `windows` の `D2D1CreateFactory`。ワークスペースの `windows` の機能に `Direct3D`・`Direct3D11`・`Dxgi` が在る（根の `Cargo.toml:57-63`）＝`Cargo.toml` に依存を足さない。
- 作れないのは「wintf の描画の木の描き方を WARP で回すこと」。§4.2 の A（画面の外へ描く口）が在れば、emo-present の WARP の装置でその口を呼べる。
- 道: (a) wintf の別の spec で画面の外へ描く口（と WARP の装置の口）を公開 (b) emo-present で WARP の装置を作り、emo-present の再生（§4.2 の B か C）を固める。(b) は wintf の描き方を固めない。

### 4.5 仮定 3: pattern の `base` と当たり判定（`hit-id-pass` まで）

- 正典（`base`）: 当たり判定もコマのサーフェスに定義されたものへ替わる。
- 今の当たり判定は面の番号だけで領域を引く（`world.rs:167-175`）。`hit_region_client` は `current_surface_id` で引き（`hit.rs:86-88`）、ID の描き先は `surface_id` が今の面と同じときだけ読む（`hit.rs:121-124`）。
- 切り替えるには: ① 今のコマ（`PatternState`）と着せ替えの ON から「今の土台の面」を求める 1 か所（合成の側・`plan.rs` の `base` の段と同じ答え）② 当たりの領域をその面で引く ③ ID の描き先の塗り直しの鍵を「面＋土台の面」にする（`presenter/hit_picture.rs`）④ SHIORI へ渡す名前もその並びから。触るのは compose の `world.rs`（または新しいファイル）と emo-present の `hit.rs`・`hit_picture.rs`。`hit.rs` はウェーブの触る表に在り、`world.rs`・`hit_picture.rs` は無い（ほかの C28 の 5 本は触らない＝重なり 0）。
- クリックの透過は合成の答えの α を読むので、`base` で替わった絵に自動で従う（追加の仕事なし）。

### 4.6 画面の描き方の分かれ目（仮定 9・11・13 の根）

- **A. element ごとに描画の木の節**（`VisualDrawContent`＋`MixMode`・入れ子の面は透明から始まる不透明度の層の節）: 開発者の 10-15 の言葉（「コマンドリストの合成ルールを個別に指定できるように」）と、`wintf-d2d-composite-rules` が固めた節ごとの規則（層の旗・切り抜き・ブレンド）をそのまま使う。後の `draw-methods-blend`（`MixMode::Blend`）・`element-clipping-option`（節の切り抜き）とも同じ形。画面の外の再生は §4.2 の A か B。
- **B. 1 本の命令の列**（要素ごとに重ね方つきの `DrawImage` を記録し、根の命令の列にする）: 画面・書き出し・offline・WARP が同じ列＝実現が 1 つ。wintf を触らない。節ごとの規則を使わない。命令の列の中の重ね方が「同じ列の先の描画」にだけ効くか（描画面の既存の画素にも効くか）は D2D の振る舞いで、**要確認**（emo の描画面は透明から始まり列のほかに中身が無いので、どちらでも答えは同じ見込み）。
- 範囲だけの描き直しはどちらも今と同じく全部（§4.3）。

### 4.7 そのほかの仮定

- **仮定 1（分割）**: Requirement 7 を後の spec へ出すと、画面が GPU になった後も 5 つの読み手が CPU の合成の答えを読み続ける＝CPU の合成が残り、新しい 5 語を CPU でも描かない限り画面と食い違う。**Requirement 7 は本 spec から切り離せない**。分けるなら、先に要る wintf の口を wintf の spec として前に出す形になる。
- **仮定 2（見本）**: 見本 8 ファイルのうち、画素の値を固定するものは WARP の見本へ、決定論・外形・着せ替え・切り詰めの同値は合成の計画（命令の列）の値で固定し直せる。CPU 専用の `blit_rect_tests.rs`・`presenter_partial_recompose_tests.rs`・`blit_transparent_alpha_tests.rs` は合成の CPU の転写と一緒に消える（陳腐化）。
- **仮定 5（入れ子の `asis`）**: 正典の「element合成の結果が一枚の画像のように扱われる」（`introduction_drawmethod`）に従い、入れ子は 1 つの層にまとめる。乗算済みの層の α を 1 にした答え＝黒の上に重ねた答えなので、黒の矩形＋`SOURCE_OVER` で決定的に描ける。
- **仮定 6・8**: 正典で読めない。開発者の方針（ukadoc で読めない点は議題の前に SSP の MCP で測る）に従い、設計の前に測る。仮定 8 の正典の文は「自明ではあるが、最初の element で指定された描画メソッドはほとんどのものが `base` と同義」で、`overlay-fast`・`reduce` を全透明のベースに式で当てた答え（何も描かない）とは食い違いうる。
- **仮定 7（オプションの並び）**: `element-clipping-option`（C30 の見込み）も同じ 17 ファイルの `Element` を直す。転記の欄だけ一緒に足せば、あちらは触り直さない。
- **仮定 10**: §2.6 のとおり付け替え不要。触る表から `balloon_background.rs` を外せる。
- **仮定 12**: element定義は完全一致・pattern定義は正規化、の食い違いは今もある。揃えると大文字の `Overlay` の element定義の行が描かれるようになる（見え方が変わる）。

## 5. 実装の道（候補）

### 道 A: 今の部品を広げる（読み手・門・計画）＋画面を節の形へ

- 読み手: `Element` に `method: DrawMethod`（とオプションの並び）を足し、`is_image_element_method` を語の全部へ広げる（`undrawn.rs` は自動で連動）。
- 合成: `BlitOp` の平らな列を「入れ子を 1 単位に持つ列」に変え、`ComposeMethod` から D2D の重ね方への 1 か所の写しを置く。CPU の転写（`blit.rs`・`blit_rect.rs`）は捨てる。
- 表示: `display.rs` を「アトラスの頁を GPU へ 1 度上げ、要素ごとの節を組む」へ、`mount.rs` を節の木の持ち主へ。
- 読み手 5 つ: §6 の議題 4・5 の答えに依る。
- ✅ wintf の節ごとの規則と後続 3 本の形がそろう ❌ 画面の外の再生の担保に wintf の口が要る（無ければ実現が 2 つ）

### 道 B: 新しい部品（emo-present の「描く記録」）＋1 本の命令の列

- 合成の段は計画（純粋な値）までにし、emo-present に新しいファイルで「計画 → 重ね方つきの D2D の命令の列」を置く。画面は根の命令の列、書き出し・offline・WARP は同じ列を画面の外で再生。
- ✅ 実現が 1 つ・wintf を触らない ❌ 節ごとの規則を使わない・後で節へ移すときにもう一度作り直す

### 道 C: 混ぜる（節で画面・記録を共有の値に）

- 計画の値（要素・位置・重ね方・入れ子の層）を 1 つにし、画面はそれを節へ、画面の外はそれを wintf の公開の口（別の wintf spec）で描く。
- ✅ 実現が 1 つ・節の規則を使う ❌ 先に wintf の spec が要る（列の順が変わる）

## 6. 設計の判断の材料（要件の討議へ渡す議題）

各項は「答えで作業が変わるもの」だけ。推奨は暫定で、決めは討議で行う。

1. **画面の描き方（節か 1 本の列か）** — 推奨: **節（§4.6 の A）**。根拠: 開発者の 10-15 の言葉が「描くものごとの合成の規則」を wintf に求め、`wintf-d2d-composite-rules` がその規則と層の旗を WARP で固めた（`draw_content.rs:8-39`・README:42）。後続の `draw-methods-blend`（`MixMode::Blend`）・`element-clipping-option`（節の切り抜き）も同じ形になる。範囲だけの描き直しはどちらも全部なので、これは分かれ目の理由にならない（§4.3）。
2. **`asis` の絵の出どころ（仮定 4）** — 推奨: **アトラスに `asis` の読みの絵を焼く（§4.1 の A）**。根拠: 透過の正規化はアトラスの持ち分（`normalize.rs` の役目）で、`asis` はその扱いの 1 種。乗算しない読みは既に在る（`decode.rs:179-184`）。触る表は emo-atlas へ広がるが、C28 のほかの 5 本と重ならない。触る表の外であることは開発者に明示する。
3. **`replace` の範囲とトリム** — 推奨: アトラスは変えず、範囲を `AtlasEntry.original`（`table.rs:58-63`）の原寸の矩形で切り、その中を `SOURCE_COPY` で上書きする（`BOUNDED_SOURCE_COPY` はトリム後の絵の矩形にしか効かない）。
4. **クリックの透過の α の出どころ（要件 7.1・7.2）** — 推奨: **開発者の決め ⑶ のとおり 1 画素をその場で読む**。wintf の当たりに「合成の入れ物の 1 画素で決める形」が要るので、**wintf の別の spec を本 spec の前に出す**（本ウェーブでは wintf に触らない約束）。名前の解決は同じ答え（`SurfaceReadback::read_pixel`）へ一緒に付け替える。⑶ を変えるなら、wintf に触らない道は「新しい絵を描いた回に全体を読み戻して 1 ビットの表を作る」（中央値 1.88 ms／512×512・`always` のアニメーションでは毎コマ）。
5. **書き出し・offline・WARP のテストの描き方（仮定 9・13）** — 推奨: **wintf の別の spec（議題 4 と同じ spec）で「描画の木を画面の外の絵へ描く口」を公開し、原寸の書き出し（`snapshot.rs` の 3 つの口と `read.rs` の `read_back`）・`--offline-dump`（emo-present の口を通す）・WARP の見本が wintf の同じ描き方を通る**。根拠: 要件 7 は「画面と同じ答え」で、実現を 1 つにするのが本筋。WARP の装置は emo-present のテストが `wintf::com::d3d11::d3d11_create_device`（pub）で作れるので、口は描き方だけで足りる。代わりの道は §4.2 の B（emo-present が再生し、実 GPU のテストで wintf の読み戻しと一致を固める）。
6. **範囲だけの描き直し（仮定 11・要件 8.2）** — 推奨: 要件 8.2 を「範囲だけで描き直す回の答えは全部の答えとバイトで一致（wintf の保証）・フレームの所要を今と比べて記録する」へ直す。「画素どおり」の申告の仕組みは **wintf の持ち分**（計画が読む）なので、要るなら議題 4・5 の wintf の spec に入れる。根拠: 今も GPU の側は毎回全部（`damage.rs:106-107`）で、CPU の重ね直しの効きは CPU の合成と一緒に消える。
7. **Requirement 7 の分割（仮定 1）** — 推奨: **分けない**（§4.7）。分けると CPU の合成が残る。分けるなら wintf の口の spec を前に出す形にする。
8. **入れ子の面を 1 枚の絵として扱う単位（要件 2.7・正典の「element合成の結果が一枚の画像のように扱われる」）** — 推奨: 合成の計画で入れ子を 1 単位（透明から始まる層）に持つ。今の `BlitOp` の平らな列（`plan.rs:59-66`）を変える。入れ子の `asis` は黒の矩形＋`SOURCE_OVER`（仮定 5）。
9. **pattern の `base` の当たり判定（仮定 3）** — 推奨: 正典どおり替える。合成の側に「今の土台の面」を答える 1 か所を置き、当たりの領域（`world.rs:167-175`）と ID の描き先の塗り直しの鍵（`hit.rs:121-124`・`hit_picture.rs`）をその答えで引く。触る表に compose の `world.rs`・emo-present の `hit_picture.rs` を足す（C28 と重なり 0）。
10. **SSP で測る 2 点（仮定 6・8）** — 推奨: 設計の前に SSP の MCP で測る。仮定 6＝2 つのアニメーションが同時に動くときの `reduce`・`replace` のベース。仮定 8＝最初の element の `overlay-fast`・`interpolate`・`reduce`・`replace`・`asis`（全透明のベース）。
11. **`blend-*` の綴りを読み手が知る形（要件 1.1・1.4）** — 推奨: 読み手（parsers）に element定義の画像の語の完全一致の述語を 1 つ置き、`blend-*` の綴りは種類の並び（`blend-`＋種類＋`-fast` の有無＋旧い別名 2 つ）から作る。compose のテストで `ComposeMethod::from_name` と全部の綴りが食い違わないことを固める（一覧を 2 か所に手で書かない）。element定義の `blend-*` の行の `warn!` は、読み手でなく合成の 1 か所（`plan_issues.rs`）から出す形にする（読み手が運ぶと `undrawn.rs` から外れるため）。
12. **element定義の語の比べ方（仮定 12）** — 推奨: 今の完全一致を保つ（完了 `element-base-method` の決め・見え方を変えない）。
13. **オプションの並びの欄（仮定 7）** — 推奨: 転記の欄だけ一緒に足す（`element-clipping-option` が同じ 17 ファイルを触り直さない）。
14. **見本の扱い（仮定 2・要件 9.6）** — 推奨: 画素の値の見本は WARP の見本へ移し、計画の値で言える見本（決定論・外形・着せ替え・切り詰め）は計画のテストに書き直す。CPU の転写のテスト（`blit_rect_tests.rs` ほか）は陳腐化として取り下げ、理由を記す。
15. **ウェーブの約束の文の直し** — 「画像の書き出しの付け替えは `presenter/read.rs` の中で閉じ」は、実際の口が `presenter/snapshot.rs` なので文を直す。`ComposedSurface`（バイトと寸法の入れ物）を残せば MCP の入口（`dump_surface.rs:300-323`・`dump_balloon.rs:55-58`）は変えずに済む。触る表から `balloon_background.rs` を外し、emo-atlas（議題 2）・compose の `world.rs`・emo-present の `hit_picture.rs`・`snapshot.rs`（議題 9・15）を足す。

## 7. 規模と危うさ

- **規模: XL**（2 週を越える）。合成の段を CPU の転写から GPU の描き方へ移し、5 つの読み手と見本を一緒に付け替える＝建て付けの変更。議題 4・5 の wintf の口を前に出すなら、その spec の規模は別に数える。
- **危うさ: High**。D2D の丸めの差（要件 8.1 の ±1）・命令の列の中の重ね方の効き方・節の木と画面の外の再生の一致・読み戻しの順序（`render_surface` の後に同じ刻みで読む口）がまだ測られていない。開発者の決め ⑶ を満たす口が wintf に無く、列の順に効く。

## 8. 設計へ持ち越す研究の項目

1. D2D の命令の列を `DrawImage` で描くとき、中に記録した重ね方が何に効くか（同じ列の先の描画だけか、描画面の既存の画素か）を WARP で測る。
2. CPU の `div255` の SourceOver と D2D の SourceOver（WARP・hardware）の差の最大を測る（要件 8.1）。
3. 原寸（k＝1）で画面の外に描いた答えと、画面の入れ物（k 倍）の関係。`dump_surface` が今と同じく原寸を返す形の確かめ。
4. 読み戻しに頼る読み手（議題 4 の代わりの道を取る場合）が、`render_surface` の後・同じ刻みの中で読める係の順序の口が wintf に公開されているか。
5. アトラスの頁を GPU へ上げる単位（頁ごとに 1 度・装置の失いで作り直す）と、今の 3 項目の合成の控え（`cache.rs`）の置き換え。
6. `UseSelfAlpha::Off` が PBGRA で読んだ色に α＝255 を当てている件（`normalize.rs:76`・半透明の画素が暗く出る疑い）。本 spec の範囲外なら起票の要否を開発者へ。
7. SSP の MCP の測り（議題 10）。

## 9. 次の段

- 要件の討議（`kiro-requirements-discussion`）で §6 の議題を決める。議題 4・5・6 の答えが「wintf の口を先に出す」なら、wintf の spec の起票（`/kiro-discovery`）と本 spec の順序を開発者と決める。
- 討議の後、`/kiro-design draw-methods-canon`。

## 10. SSP の MCP での測り（2026-11-27・要件の討議の議題 6）

- **やり方**: SSP 付属の emily4 に検体のシェル（`shell\areka-dm-probe`・`seriko.use_self_alpha,1`）を一時的に置き、`\![call,ghost,Emily/Phase4.5]` で呼んで `\![change,shell,…]` で切り替えた。`mcp__ssp__dump_surface` の絵（30×10）を、列 x=5／15／25 の画素で読んだ（BGRA でなく RGBA・乗算していない値）。測り終えてシェルを `Phase4.5[新]` へ戻し、Emily を閉じ、検体を消した。
- **検体の絵**: `base.png`＝赤の不透明｜赤 α128｜全透明。`blue.png`＝青の不透明。`blue60.png`＝青 α153。`green_r.png`＝右の 3 分の 1 だけ緑の不透明。`bluekey.png`＝24 ビット・左上の透過色（マゼンタ）｜青。
- 注: `use_self_alpha` を書かない検体では、α を持つ PNG も左上の色を透過色として抜いた（全部が青の絵は丸ごと透明）。

### 10.1 仮定 8: 最初の element（ベースが空）

| 面 | element0 | x=5 | x=15 | x=25 |
|---|---|---|---|---|
| 1 | `overlay` blue | 0,0,255,255 | 同 | 同 |
| 2 | `overlay-fast` blue | 0,0,255,255 | 同 | 同 |
| 7 | `overlayfast` blue | 0,0,255,255 | 同 | 同 |
| 3 | `interpolate` blue | 0,0,255,255 | 同 | 同 |
| 4 | `replace` blue60 | 0,0,255,153 | 同 | 同 |
| 5 | `reduce` blue60 | 0,0,255,153 | 同 | 同 |
| 8 | `base` blue | 0,0,255,255 | 同 | 同 |
| 6 | `asis` bluekey | **0,0,0,0**（透過色が抜けた） | 0,0,255,255 | 0,0,255,255 |

- SSP は最初の element を、語に依らず `base`（新規レイヤをそのまま写す）として描く。`overlay-fast`・`reduce` も式どおり（何も描かない・掛け算）にならない＝正典の「ほとんどのものが `base` と同義」と一致。
- **`asis` は正典と食い違う**: 正典は「ただし `asis` で透過色がそのまま表示される点は有効」と書くが、SSP 2.x の最初の `asis` は透過色を抜いた。2 つ目の element の `asis`（下の面 15）は透過色を表示した。

### 10.2 2 つ目の element（ベース＝`base.png`）の確かめ

| 面 | element1 | x=5（ベース不透明） | x=15（ベース α128） | x=25（ベース透明） |
|---|---|---|---|---|
| 11 | `overlay-fast` blue | 0,0,255,255 | 0,0,255,128 | 0,0,0,0 |
| 12 | `interpolate` blue | 255,0,0,255 | 64,0,126,255 | 0,0,255,255 |
| 13 | `replace` blue60 | 0,0,255,153 | 0,0,255,153 | 0,0,255,153 |
| 14 | `reduce` blue60 | 255,0,0,153 | 255,0,0,77 | 0,0,0,0 |
| 15 | `asis` bluekey | 255,0,255,255（透過色が出る） | 0,0,255,255 | 0,0,255,255 |

- `overlay-fast`・`replace`・`reduce`・`asis` は正典の文と一致（`reduce` の α は 128×0.6＝76.8→77）。
- `interpolate` の半透明の所は、正典の式（ベースを新規レイヤの上に重ねる＝赤 128・青 127 の見込み）に対して赤が 64 と半分になった。SSP の実装の癖とみて写さない（正典の文に従う）。

### 10.3 仮定 6: 同時に動く 2 つのアニメーション

| 面 | アニメーション（ID の順） | x=5 | x=15 | x=25 |
|---|---|---|---|---|
| 20 | 0: `overlay` green_r → 1: `reduce` blue60 | 255,0,0,153 | 255,0,0,77 | **0,255,0,153** |
| 21 | 0: `reduce` blue60 → 1: `overlay` green_r | 255,0,0,153 | 255,0,0,77 | **0,255,0,255** |
| 22 | 0: `reduce` blue60 だけ | 255,0,0,153 | 255,0,0,77 | 0,0,0,0 |

- SSP は、同時に動くアニメーションを ID の順に、それまでに描いた答えの上へ重ねる。後の `reduce` のベースレイヤには、先に描いたアニメーションのコマが入る（面 20 の x=25）。前のコマの答えは次のコマへ持ち越さない（`always` の 1 コマの繰り返しで、`reduce` が積もらない＝面 22 は 153 のまま）。

## 11. 設計の段の調べ（2026-11-27・`/kiro-spec-design -y`）

- **Discovery の種類**: full（複雑な統合）。コードベースを 3 つの調べ（wintf の口・compose・emo-present／atlas／parsers／offline）に分けて読んだ。D2D の振る舞いで読めない所は Web で決めず、実装の最初に WARP で測る研究の項目（design.md の R1〜R5）へ回した。SSP の MCP は使っていない（調整役の指示）。
- **使った決まり**: `.claude/skills/kiro-spec-design/rules/` の design-principles・design-discovery-light/full・design-synthesis・design-review-gate。steering は `tech.md` の Testing（合成の画素は WARP で ±1・差分の描き直しは同じ装置でバイト一致・GPU の在りかは実 GPU）と `structure.md`（`offline_dump/` は wintf を直に使わない・ファイルは 1,000 行の手前）。

### 11.1 調べた事実（ギャップ分析に足すもの）

| 事実 | 根拠 |
|---|---|
| 不透明度の層は `Visual.opacity < 1.0` のときだけ積まれ、透明から始まり SourceOver で閉じる。1.0 では層が無い＝入れ子の面を 1 枚にまとめる口は wintf に無い | `crates/wintf/src/ecs/graphics/visual/draw/builder.rs:184-195`・`emit.rs:211` |
| 節の中身は命令の列だけ（絵の型は無い）。命令の列は GraphicsCore の装置で作り閉じてから入れる | `draw_content.rs:32-104` |
| 描画の木を画面の外で描く手順は、wintf の私有のテストの足場 `render_tree_to_pixels`（`fill_world_visuals` → `build` → `emit`）にもう在る。使い捨ての World で動く | `visual/draw/emit_test_support.rs:322-345` |
| `GraphicsCore::new` は hardware だけ。WARP の装置は私有の足場（`warp_device`）にだけ在る | `core.rs:144,270-283`・`emit_test_support.rs:60` |
| 入れ物の 1 画素は `pixel_in`（`pub(crate)`）で `&World` からも読める。同じ座標・同じ世代は控え | `readback.rs:99`・`composite.rs:412-440` |
| 当たりの腕は `hit_test_entity_ex` の `match mode`（`AlphaMask` の腕 :357）。`WM_NCHITTEST` とクリックの透過の係の両方が UI スレッドで同期に呼ぶ | `hit_test/mod.rs:68,314-362`・`window_proc/mouse_move.rs:62`・`clickthrough/controller.rs:161` |
| 自動で上げた部分木は親の入れ物の読み戻しに入らない＝入れ物の α で当たりを決めるなら、上げた子孫の入れ物も重ねて読む | `visual/draw/README.md:188` |
| 名前の解決の呼び手 `resolve_hit_owned` はもう `&World` を持つ | `crates/areka-input/src/lib.rs:405-413` |
| `last_shown` は `Option<(u32, Option<&ComposedSurface>)>` を返し、`dump_surface.rs` と `dump_balloon.rs` は `None` の枝で `compose_last_shown` を呼ぶ | `presenter/snapshot.rs:47`・`crates/areka/src/mcp/dump_surface.rs:300-323`・`dump_balloon.rs:55-58` |
| `Composer::compose` の本番の呼び手は snapshot 2・`areka-placement/src/measure.rs:452`（寸法だけ）・offline 1。例 4 本とテストが続く | grep |
| `ComposeError` は `#[non_exhaustive]`（足しても呼び手の `match` は壊れない） | `error.rs:26` |
| `UseSelfAlpha::Off` は `32bppPBGRA` で読んだ乗算済みの色に α＝255 を当てる＝半透明の画素が暗く出る（ギャップ分析の疑いは本当） | `decode/wic_arm.rs:139-141`・`normalize.rs:76,135-145` |
| 網羅台帳の `blend-*` は 27 種＋`blend-dither`（compose の `BlendKind` は 19 種） | `doc/ukadoc-coverage/ledger/assets.toml` |
| 画素の上限 `cap_pixels`・`PIXEL_FACTOR` は `blit.rs` に在り `build_plan` が呼ぶ＝転写を捨てても上限は残す | `blit.rs:103-121`・`plan.rs:754` |

### 11.2 形の比べ

| 道 | 中身 | 良い所 | 危うい所 |
|---|---|---|---|
| **計画 → 節 → wintf の 1 つの描き方（採用）** | compose は純粋な計画、emo-present が節へ写し、画面は描く係、画面の外は使い捨ての World＋wintf の画面の外の口 | 描き方が 1 つ（7.8）・節ごとの規則と後続（blend・切り抜き）が同じ形・compose は純粋のまま | wintf の口が 4 つ要る（まとめの節が要件の境界の外） |
| 1 本の命令の列（ギャップ分析の道 B） | 要素ごとの重ね方つき `DrawImage` を 1 本に記録 | wintf に触らない | 開発者の決め（element ごとに節）と外れる・中の重ね方の効き方が未測 |
| 画面は節・書き出しは全体の読み戻し | 画面の入れ物を `read_all` | 口が少ない | 入れ物は k 倍で原寸でない（7.3）・隠した面と単体の面は描けない |

### 11.3 決め（design.md に書いたものの訳）

- **入れ子の面のまとめ**: 正典「element合成の結果が一枚の画像のように扱われる」と開発者の決め（透明から始まる 1 枚）から、wintf にまとめの節を足す。中に `SourceOver` でない項が無く、指す語も `SourceOver` なら平らに並べる（結合則で同じ答え）。
- **最初の項の読み替え**: 透明の上の `SourceOver` は写しと同じなので、`asis` 以外を `SourceOver` へ読み替えるだけで「`base` と同義」（§10.1）になる。`asis` は `asis` 用の絵（α＝255・原寸）なので透過色が出る（正典どおり・SSP と違う所）。
- **`replace` の範囲**: `SOURCE_COPY` は絵を今の切り抜きまで透明で広げるので、原寸の矩形の切り抜きの節の中に置く。
- **当たり**: 入れ物の 1 画素を wintf の 1 関数（`surface_alpha_at`）で読み、当たりの腕と名前の解決が同じ関数を通る。上げた子孫の入れ物も重ねる。
- **書き出しの型**: 合成メモの項に書き出しの控え（`OnceCell`）を持ち、`last_shown` の型を保つ（`dump_surface.rs` を触らない）。
- **WARP**: `warp_d2d_device` を wintf で公開し、画面の外の口に任意の装置を渡す。`GraphicsCore` に WARP の作り方は足さない（範囲だけ＝全部のバイト一致は同じ装置の実 GPU で確かめる）。

### 11.4 synthesis

- **一般化**: 画面・`dump_surface`・`--offline-dump`・WARP のテストは「計画を節にして描く」の 1 つの問題。`DrawNodes` の 1 関数と wintf の 1 つの描き方にまとめた。クリックの透過と名前の解決も「画面に出ている α」の 1 問題として `surface_alpha_at` にまとめた。
- **作るか使うか**: D2D の重ね方（`SOURCE_ATOP`・`DESTINATION_OVER`・`DESTINATION_IN`・`SOURCE_COPY`）は正典の式とそのまま一致するので使う。`asis` と `use_self_alpha,0` は既存の乗算しない読み（`decode_unmultiplied`）を使う。WARP の装置は既存の `d3d11_create_device` を使う。
- **削ったもの**: CPU の転写と範囲の重ね直し・表示の絵の席・α マスクの回し・毎回の `CreateBitmap`。まとめの節は要るときだけ。「画素どおり」の申告は測りで重くなったときまで足さない。

### 11.5 危うさ

- まとめの節の `SourceOver` 以外の閉じ方（R1）・D2D と CPU の丸めの差（R2）・更新の所要（R3）・当たりの読みの待ち（R4）・k が整数でないときの継ぎ目（R5）。どれも実装の最初に測り、外れたら止めて報告する。

## 12. SSP の MCP での測り（2026-11-27・設計の討議の前・設計の仮定 A〜C）

やり方は §10 と同じ（emily4 に検体のシェルを一時的に置き、測り終えてシェルを戻し、Emily を閉じ、検体を消した）。絵は §10 の検体（`base.png`・`blue60.png`・`green_r.png`＝面 102 の絵）。

| 面 | 書き方 | x=5 | x=15 | x=25 | 読み |
|---|---|---|---|---|---|
| 33 | element0 が無い絵 `missing.png`・element1 が `reduce` blue60 | 0,0,255,153 | 同 | 同 | 描くものが無い項は「最初」に数えない。`reduce` が最初の項になり、`base` と同じに写した |
| 30 | animation0 `base`→面 100・animation1 `base`→面 102（同時・`always`） | 0,0,0,0 | 0,0,0,0 | 0,255,0,255 | 置き換えが重なると、ID の順の後（面 102）が勝つ |
| 31 | animation0 `base`→`-1` | 255,0,0,255 | 255,0,0,128 | 0,0,0,0 | 置き換えない（土台の絵のまま） |
| 32 | animation0 `base`→面 999（無い面） | 255,0,0,255 | 255,0,0,128 | 0,0,0,0 | 置き換えない（土台の絵のまま） |
| 34 | animation0 `base`→面 102・XY＝(7,3) | 0,0,0,0 | 0,0,0,0 | 0,255,0,255 | XY を無視した（ずらすと x=25 は透明になるはず） |

- 設計の仮定 A は改める: 「最初の element」は、描くものが在る最初の項（無い絵の項は数えない）。
- 仮定 B（後が勝つ）・仮定 C（無い面・`-1` は置き換えない）・`base` の XY を無視する（正典）は、SSP と一致した。

## 13. 設計の討議の決め（2026-11-27・全議題を推奨案で承認）

1. k が整数でないとき: emo の描画面の下を原寸のまとめの節 1 つで包み、k 倍する（今の「原寸で合成してから拡大」を保つ・まとめの中は自動で上げない＝画面の絵は入れ物 1 枚）。検証の指摘 1・3 を解く。
2. wintf のまとめの節（`VisualGroup`）を足す（4 つ目の口・要件の境界へ足した）。
3. 名前の解決に `&World` を渡すための `areka-runtime/src/hit_region.rs`・`areka-input/src/lib.rs`、CPU の合成の口を外すための `areka-placement/src/measure.rs`・例・テストの支えに触る（C28 のほかの 5 本は触らない＝重なり 0）。
4. バルーンの絵も節で描く（同じ `apply_show` を通る）。
5. 動く絵の `asis` と `use_self_alpha,0` は、動く絵の読みの腕に乗算しない各コマの読みを足して同じ規則で作る。

## 14. 移す前の更新の所要（2026-11-27・タスク 1.1・要件 8.2・9.3・研究の項目 R3）

移した後（タスク 8.3）は、この節の手順を変えずに測り直し、下の表と並べる。

### 14.1 検体と条件

- **検体**: emo2 の TargetId(0)（本体）が起動時に出す surface1000 のまばたき（`animation1400`。`bind+random,4`・コマ 1412→1411（150ms）→1410（22ms））と、同じ枠のまばたきの別の形 `animation1402`（ジトー目のときに bind で替わる）。tasks.md・design.md の「surface0 のまばたき」はこれを指す。emo2 の surface0 は `element0,overlay,surface0.png` の 1 枚だけで、まばたきが無い。
- **k**: この機械のモニタは 2 面とも 192 dpi（200%）。Windows の表示設定は変えず、検体の写しの shell `descript.txt` の末尾に `seriko.dpi` を 1 行足して k を作る。
  - k＝1: `seriko.dpi,192`（記録の `k_shell_ratio=ScaleRatio { num: 1, den: 1 }`）。
  - k≈1.25 の代わり: `seriko.dpi,154`＝**k＝192/154＝96/77≈1.2468**（記録の `k=1.246753215789795`・`k_shell_ratio=ScaleRatio { num: 96, den: 77 }`）。192 dpi のモニタでは、ちょうど 1.25 になる整数の `seriko.dpi` が無い（153.6 が要る）。ちょうど 1.25 は表示の拡大率を 125%（120 dpi）にすれば作れるが、Windows の表示設定を変えることになるので取らない。
  - バルーンは既定の 96 dpi のまま（`k_balloon=2.0`）。
- **実行体**: `target\release\areka.exe`（release・2026-11-27 21:19 に建てた。そのときの作業木は HEAD `f7d1336b` に、別のタスクの未コミットの wintf の変更（`offscreen.rs` ほか。描く係の道には触らない）が乗った状態）と、`target\release\shiori-host32-helper.exe`（i686 の release を建てて写した。これが無いと SHIORI の接続が失敗し、起動から約 10 秒で exit=1 になる）。実 SHIORI があり、発話もある。
- **回数**: k ごとに 3 回、1 回 60 秒（`AREKA_APP_SMOKE_EXIT_MS=60000`）。6 回とも exit=0。各回の前に cargo/rustc が無いことを待って確かめた。走っている間も 2 秒ごとに見て、6 回ともビルドとの重なりは無かった（`build_overlap=False`）。
- **環境変数**（`crates/real-run-kit/src/launch.rs` の `child_command` と同じ）: 回ごとに空の `AREKA_ROOT`・`AREKA_PROFILE_DIR`（`target\dmc-perf\run-<k>-<i>\root`・`\profile`）、`AREKA_MCP_PORT=47391`、`AREKA_NO_ALERT=1`、`NO_COLOR=1`、`PASTA_DEBUG=0`、`RUST_LOG=info,areka_emo_present=debug,wintf::damage=debug`。引数はゴーストの根とバルーンの根の絶対パス。

### 14.2 読む記録の行

- `perf(apply_show): 段階別計時`（`areka_emo_present::presenter::timing`・debug）のうち `target_id=TargetId(0) surface_id=1000` の行。欄は `t_compose_us`（CPU の重ね直し）・`t_mask_us`・`t_upload_us`（原寸の絵の上げ＋表示の記録）・`t_total_us`（`apply_show` 全体）。`cache_hit` の真偽で分ける。
- その行の後の最初の `[render_surface] damage`（target `wintf::damage`・debug）のうち、本体の描画面のもの。欄は `compose_us`・`present_us`・`mode`・`reason`。`entity` は本体・相方・バルーンとも `emo-surface` なので、`surface_area` で本体を選ぶ（表示の行の `scaled_w×scaled_h` との差が 1% 以内のもの）。
  - k≈1.2468 では、表示の行が 476×682（324,632）なのに対し、wintf の描画面は 325,314（477×682）。幅の丸めが 1 画素違う。
- **まばたきのコマ**: `seriko: loop 抽選発火` の行（`scope="0"`・`animation_id=1400` か `1402`）から 250ms 以内の上の適用。それ以外の TargetId(0)/1000 の適用（発話の表情の bind の替え）は「表情の替え」として別に数える。

### 14.3 移す前の所要（中央値・µs）

| k | 種類 | cache_hit | 件数 | `t_compose_us` | `t_mask_us` | `t_upload_us` | `t_total_us` | wintf `compose_us` | wintf `present_us` | wintf の描き方 |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | まばたき | false | 13 | 307 | 218 | 263 | 2,249 | 99 | 60 | 全部（`root_list`） |
| 1 | まばたき | true | 3 | 0 | 0 | 0 | 528 | 146 | 109 | 全部（`root_list`） |
| 1 | 表情の替え | false | 19 | 1,760 | 309 | 318 | 3,509 | 84 | 68 | 全部（`first`・`root_list`） |
| 192/154≈1.2468 | まばたき | false | 14 | 252 | 182.5 | 233.5 | 2,349 | 85.5 | 53 | 全部（`root_list`） |
| 192/154≈1.2468 | まばたき | true | 24 | 0 | 0 | 0 | 298.5 | 95 | 66 | 全部（`root_list`） |
| 192/154≈1.2468 | 表情の替え | false | 18 | 1,358 | 196.5 | 248 | 2,417.5 | 80 | 74.5 | 全部（`first`・`root_list`） |

- まばたきの 1 コマの更新の所要（`t_total_us`＋wintf の `compose_us`＋`present_us` の中央値の和）は次のとおり。8.3 はこの値と比べる。
  - k＝1: 外れ（cache_hit=false）約 2,408µs・当たり（cache_hit=true）約 783µs。
  - k≈1.2468: 外れ約 2,488µs・当たり約 460µs。
- wintf の側は、まばたきの回も毎回「全部」を描いた（`mode="full"`・`reason="root_list"`）。範囲だけになった回は 0 件。§3 の 8.2 の行（今も wintf は毎回全部を描く）の実測の裏付けになる。
- 発火の数は 3 回の走行の和で、k＝1 が 7 回、k≈1.2468 が 13 回（1400 と 1402 の和）。件数が少ないので、8.3 も同じ 60 秒×3 回で測り、件数を並べて書く。

### 14.4 測り直しの道具（そのまま写して使う・作業木の外に置く）

検体の写しの作り方（PowerShell）: 検体は `cargo run -p sample-ghost-kit --bin nar-sample-path -- emo2` が刷る `folder=`。

```powershell
$w='<作業木の絶対パス>'; $src="$w\target\nar-samples\manual\emo2\ghost\emo2"
foreach($k in @(@('k1',192),@('k125',154))){ $d="$w\target\dmc-perf\$($k[0])\emo2"; if(Test-Path $d){Remove-Item -Recurse -Force $d}; New-Item -ItemType Directory -Force (Split-Path $d)|Out-Null; Copy-Item -Recurse $src $d; Add-Content -Encoding utf8 "$d\shell\master\descript.txt" "`r`nseriko.dpi,$($k[1])" }
```

走らせ方（PowerShell・`-Runs 3 -ExitMs 60000`）:

```powershell
param([int]$Runs = 3, [int]$ExitMs = 60000, [string[]]$Ks = @('k1','k125'))
$w = '<作業木の絶対パス>'; $exe = "$w\target\release\areka.exe"; $base = "$w\target\dmc-perf"
foreach ($k in $Ks) { for ($i = 1; $i -le $Runs; $i++) {
    while (Get-Process cargo, rustc -ErrorAction SilentlyContinue) { Start-Sleep 10 }
    $run = "$base\run-$k-$i"; if (Test-Path $run) { Remove-Item -Recurse -Force $run }
    New-Item -ItemType Directory -Force "$run\root", "$run\profile" | Out-Null
    $ghost = "$base\$k\emo2"
    $env:AREKA_ROOT = "$run\root"; $env:AREKA_PROFILE_DIR = "$run\profile"; $env:AREKA_MCP_PORT = '47391'
    $env:AREKA_NO_ALERT = '1'; $env:NO_COLOR = '1'; $env:PASTA_DEBUG = '0'; $env:AREKA_APP_SMOKE_EXIT_MS = "$ExitMs"
    $env:RUST_LOG = 'info,areka_emo_present=debug,wintf::damage=debug'; Remove-Item env:AREKA_USER_ROOT -ErrorAction SilentlyContinue
    $p = Start-Process -FilePath $exe -ArgumentList "`"$ghost`"", "`"$ghost\balloon\emo2-kakukaku`"" -RedirectStandardOutput "$run\run.log" -RedirectStandardError "$run\run.stderr.log" -NoNewWindow -PassThru
    $overlap = $false; $deadline = (Get-Date).AddMilliseconds($ExitMs + 60000)
    while (-not $p.WaitForExit(2000)) { if (Get-Process cargo, rustc -ErrorAction SilentlyContinue) { $overlap = $true }
        if ((Get-Date) -gt $deadline) { Stop-Process -Id $p.Id -Force; break } }
    Write-Output "done $k $i exit=$($p.ExitCode) build_overlap=$overlap" } }
```

読み方（Python 3 の標準だけ・`python <これ>.py k1 k125`）:

```python
import re, sys, statistics as st, glob, os
from datetime import datetime
base = r'<作業木の絶対パス>\target\dmc-perf'
kv = re.compile(r'(\w+)=("[^"]*"|\S+)')
def fields(l): return {k: v.strip('"') for k, v in kv.findall(l)}
def ts(l): return datetime.fromisoformat(l[:26])
def med(xs): return st.median(xs) if xs else None
for k in sys.argv[1:]:
    rows = {'blink': [], 'other': []}; fires = 0
    for log in sorted(glob.glob(os.path.join(base, f'run-{k}-*', 'run.log'))):
        L = open(log, encoding='utf-8', errors='replace').read().splitlines()
        area = next(int(f['scaled_w']) * int(f['scaled_h']) for f in (fields(l) for l in L if '表示・マスクを更新' in l and 'TargetId(0)' in l))
        last_fire = None
        for i, l in enumerate(L):
            if '抽選発火' in l and 'scope="0"' in l and re.search(r'animation_id=14(00|02)\b', l):
                last_fire = ts(l); fires += 1
            if 'perf(apply_show)' in l:
                f = fields(l)
                if f.get('target_id') != 'TargetId(0)' or f.get('surface_id') != '1000': continue
                d = next((fields(m) for m in L[i + 1:i + 40] if '[render_surface] damage' in m
                          and abs(int(fields(m)['surface_area']) - area) <= area // 100), None)
                kind = 'blink' if last_fire and 0 <= (ts(l) - last_fire).total_seconds() <= 0.25 else 'other'
                rows[kind].append((f, d))
    print(k, 'fires', fires)
    for kind, rs in rows.items():
        for hit in ('false', 'true'):
            sel = [(f, d) for f, d in rs if f['cache_hit'] == hit]
            if not sel: continue
            ds = [d for _, d in sel if d]
            out = {c: med([int(f[c]) for f, _ in sel]) for c in ('t_compose_us', 't_mask_us', 't_upload_us', 't_total_us')}
            out.update(n=len(sel), paired=len(ds), compose_us=med([int(d['compose_us']) for d in ds]),
                       present_us=med([int(d['present_us']) for d in ds]), modes=sorted({(d['mode'], d['reason']) for d in ds}))
            print(k, kind, 'cache_hit=' + hit, out)
```

- 8.3 では「移した後」の `areka.exe` を建て直してから、同じ写し・同じ道具・同じ回数で測る。移した後は `t_compose_us`（CPU の重ね直し）が無くなる見込みなので、比べるのは 1 コマの和（`t_total_us`＋`compose_us`＋`present_us`）。
- この回の記録（`target\dmc-perf\run-{k1,k125}-{1..3}\run.log`）は作業木の `target\` の下にあり、消えることがある。表の数が正本になる。

## 15. まとめの節を閉じる仕組み（2026-11-27・タスク 2.3・研究の項目 R1）

### 15.1 測り（WARP・16×16 の的・`DrawImage`・`LINEAR`）

**中の重ね方が中だけに効くか**（親＝全面の赤。中身＝青 (0,0)-(8,8) の後に (4,4)-(12,12) を `DESTINATION_IN`。約束どおりなら 1 枚は青の (4,4)-(8,8) だけ）。中身を 1 本の命令の列に記録し、親へ 1 度 `DrawImage` した答え:

| 閉じる重ね方 | (2,2) | (6,6) | (10,10) | (14,14) | 約束どおりか |
|---|---|---|---|---|---|
| `SOURCE_OVER` | 赤 | 青 | 赤 | 赤 | はい |
| `SOURCE_ATOP` | 赤 | 青 | 赤 | 赤 | はい |
| `DESTINATION_OVER` | 赤 | 赤 | 赤 | 赤 | はい |
| `DESTINATION_IN` | 透明 | 赤 | 透明 | 透明 | はい |
| `SOURCE_COPY` | 透明 | 青 | 透明 | 透明 | はい |

命令の列でも、中の `DESTINATION_IN` は親の画素に届かない（D2D は命令の列を 1 枚の絵として扱う）。

**中を局所の解像度で描くか**（中身＝赤 (0,0)-(1.5,4)。4 倍の変換で描く。原寸で描いてから拡大すれば、物理 (2,8) の α は 239・(5,8) は 143 になる）:

| 仕組み | (2,8) の α | (5,8) の α | 読み |
|---|---|---|---|
| 命令の列を 4 倍で `DrawImage`（`SOURCE_OVER`・`DESTINATION_OVER`・`SOURCE_ATOP`） | 255 | 255 | 的の解像度で描き直す（縁は物理 x＝6 で切れる） |
| 作業用の絵（局所の 1 DIP＝1 画素）を 4 倍・`LINEAR` で `DrawImage` | 239 | 143 | 原寸で描いてから拡大する |

### 15.2 決め

- **作業用の絵を通す**。命令の列は「透明から始まる・中の重ね方は中だけ」を満たすが、変換つきで描くと中の命令を的の解像度で描き直すので、「中は節の局所の座標の解像度で描く」（原寸で合成してから k 倍・要件 8.1・§13 の決め 1）を満たせない。作業用の絵は 3 つの約束を全部満たす。
- 作業用の絵は、同じ装置の別の DC で、透明に塗った絵（範囲＝部分木の外接矩形と有効な矩形の交わりを節の局所へ写し、整数へ外向きに丸めた矩形・`B8G8R8A8` 乗算済み・96 DPI）へ中身を描き、節の変換を当てて `LINEAR`・`mix` で親へ `DrawImage`（ブレンドは `BlendImage`）する。今の DC の的を付け替えない（外側に積んだ層と切り抜きを壊さない）。
- **`SOURCE_OVER` で、節の局所→物理の変換が整数の平行移動のときだけ**、透明から始まる不透明度 1 の層（`D2D1_LAYER_OPTIONS1_NONE`）で描く。局所の画素の格子が的の格子と重なるので答えは作業用の絵と同じで、絵を作らずに済む（設計の「`SOURCE_OVER` なら不透明度 1 の層で足りる」は、変換が拡大・回転・端数の平行移動を持つと原寸の約束を満たせないので、この条件に絞った）。
- `SOURCE_OVER` でない重ね方のまとめは、描く中身の件と同じく、1 枚を今の軸に沿う切り抜きまで透明で広げて重なる（作業用の絵の範囲の取り方に依らない）。
- 差分の描き直しは、まとめを含む描く列を項にしない（`entries` が `None`＝その描画面は毎回全部描く）。範囲の規則はタスク 2.4 で決める。

### 15.3 残る限り

- まとめの節の変換が回転を持ち、中に世界で軸に沿う矩形の切り抜きが在る形（子が逆に回っているとき）は、中の `PushAxisAlignedClip` が局所では軸に沿わないので外接矩形で切る。emo の使い方（拡大と平行移動だけ）では起きない。
- `VisualGroup` を外しただけ（ほかに何も変えない）の回は版が上がらない。節を作り直す使い方（タスク 6.2）では親子の変化で組み直される。

## 16. `overlay` だけの面の CPU と WARP の差（2026-11-28・タスク 6.5・要件 8.1・研究の項目 R2）

### 16.1 測り方

- 検体は emo2 の実物の絵（`load_shell_target`＋`WicDecoderArm`・焼きの誤り 0 件）。同じ面の表と索引表を、今の CPU の合成（`Composer::compose`）と画面の外の描き手（`OffscreenRenderer::warp()`）で描き、色の 4 成分ごとの差の最大と、差のある画素の数を数えた。
- テストは `crates/areka-emo-present/src/offscreen_emo2_tests.rs` の `emo2_overlay_only_warp_differs_from_the_cpu_compose_by_at_most_one`（差の最大 1 以内を固める）。数は `cargo test -p areka-emo-present --lib offscreen::emo2_tests -- --nocapture` の `R2` の行で読む。
- 着せ替えの既定は `descript.txt` の `bindgroup*.default,1`（1100・1207・1302・1500・1800）。まばたきは既定に 1400 を足し、`PatternState` の 1400 の欄にコマ（1412・1411・1410）を載せた。

### 16.2 答え

| 場面 | 原寸 | α>0 の画素 | 半透明の画素 | 差の最大 | 差のある画素 |
|---|---|---|---|---|---|
| surface0 | 434×687 | 140,525 | 11,860 | 0 | 0 |
| surface1000 着せ替え（既定） | 382×547 | 81,829 | 4,446 | 0 | 0 |
| surface1000 着せ替え（1101・1200・1305・1501） | 382×547 | 71,842 | 4,041 | 0 | 0 |
| まばたき 1412 | 382×547 | 81,829 | 4,353 | 0 | 0 |
| まばたき 1411 | 382×547 | 81,829 | 4,353 | 0 | 0 |
| まばたき 1410 | 382×547 | 81,829 | 4,353 | 0 | 0 |

- 6 場面とも全部の画素がバイトで一致した（半透明の縁が重なる所を含む）。CPU の `div255` の SourceOver と WARP の丸めは、等倍・整数の位置の `overlay` では差が出ない。要件 8.1 の「差 1 以内」は余裕をもって満たす。
- まばたきの 3 コマは互いに違う絵になることもテストが確かめている（目の部品は不透明の顔の内側なので α>0 の数は同じ）。

### 16.3 移した見本（要件 9.6・design の取り下げと移しの表）

| 移す前（`areka-emo-compose`） | 移した先（`areka-emo-present/src/offscreen_emo2_tests.rs`・WARP・±1） |
|---|---|
| `golden_tests_surface0_base_tests.rs` の `surface0_element0_single_layer_equals_inserted_image` | 同名。挿入した 37×29 の傾斜の絵と各成分 ±1 |
| `golden_tests_trim_equivalence_tests.rs` の `surface0_trimmed_composite_equals_untrimmed_placement` | 同名。切り詰めの記録（原寸・`trim_offset`・uv）を確かめ、原画像と各成分 ±1 |
| `golden_tests_surface1000_bind_tests.rs` の `surface1000_bind_count_overlap_point_sampling` | 同名。部品だけが覆う点の色と α、不透明の画素の数の増え方 |

- 移す前の 2 ファイルは消し、`golden_tests_surface1000_bind_tests.rs` には外形と空の着せ替えのテスト（計画の値へ書き直す側・タスク 7.6）だけを残した。
- 差を測るテストは CPU の合成を呼ぶので、CPU の合成の口を外すタスク 7.6 で一緒に取り下げる。
