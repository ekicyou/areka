# Brief: areka-P0-mcp-dump-images

> 2026-09-29 `/kiro-discovery` で起票。SSP MCP 移植の **3 段目（個別のツール）**の 1 本。並走の相手と干渉条件は `.kiro/steering/roadmap.md`「SSP MCP の移植」節。事実の正本は [doc/ssp-mcp/survey.md](../../../doc/ssp-mcp/survey.md)。file:line は起票時（main `c3876110`）＝着手時に引き直す。

## Problem

AI エージェントは台本の結果を「目で」確かめたい。SSP の `dump_surface`（今の見た目、または指定した surface 単体）と `dump_balloon`（最後に描いたバルーン）は透過 PNG を返す。areka の窓は GPU 合成で、OS のスクリーンショットでは撮れない（記憶 areka-gpu-window-screenshot-readback）＝読み戻しの口が要る。

## Current State

- **今の見た目**: `EmoPresenter::read_back(target)`（`crates/areka-emo-present/src/presenter/read.rs` 230 行目付近）が合成済みの premultiplied BGRA を等倍で返す。`current_surface_id` も同ファイル。UI スレッドでしか呼べない。本番の包み `read_back_target`（`emo2_boot/frame/wiring.rs` 274 行目付近）は `#[cfg(test)]`。
- **指定した surface 単体**: emo-compose で画面外に合成できる（`ComposedSurface`・`areka-emo-compose/src/composed.rs`）。
- **バルーン**: 背景は emo-present の target、文字は別の D3D11 スワップチェーン（`areka-emo-text/src/surface.rs` 326 行目付近の `read_back()`＝前面バッファの写し）。2 つを重ねる必要がある。emo-text の UI アクタの inbox（`TextMsg`）は `Cue` と `Close` だけ＝返事付きの変種が要る。
- PNG の符号化は無い（WIC は `windows` クレートで使える＝新しい依存は不要）。base64 も無い（数十行で書ける）。
- SSP の結果（survey §3）: 本文 `OK:scope 0, surface 3 as currently shown (with running animations and dressups, before scaling and transparency)`＋PNG、バルーンは `OK:balloon of scope 0 as last drawn (before scaling and transparency; kept even if the balloon is hidden now)`。失敗 `NG:No such surface ID. Check get_expression_table tool`・`NG:No such scope in this ghost`。

## Desired Outcome

- `dump_surface(scope?, surface?, ghost_name?)`: 省略で今の見た目（アニメーションと着せ替え込み・拡大と半透明の前）、指定でその surface の初期状態を単体で、透過 PNG で返す。
- `dump_balloon(scope?, ghost_name?)`: そのスコープのバルーンを最後に描いた内容で（隠れていても残っていれば）返す。
- 画像は `{type:"image", mimeType:"image/png"}` の content、本文は SSP と同じ文言。

## Approach

UI スレッドの処理で読み戻し → premultiplied から straight α へ戻す → WIC で PNG → base64。指定 surface は emo-compose で画面外に合成する。バルーンは背景と文字の 2 枚を重ねる。

## Scope

- **In**: 2 ツールの中身・読み戻しの本番の口・emo-text の返事付き変種・PNG 符号化と base64・決定論テスト（GPU を使うテストは cargo test の定石＝記憶 areka-no-ci-gpu-tests-in-cargo-test、画素の検証は画面外の D2D ターゲット＝記憶 gpu-draw-verification-offscreen-d2d-target）。
- **Out**: 台本の `\![execute,dumpsurface,…]`／`\![execute,dumpballoon,…]`（ファイルへ書き出す・出力先の制限・完了イベント。ukadoc 網羅台帳では `absent`・E2）。本 spec の部品を使って後で足せる＝α 後の棚卸で起票を判断する。

## Boundary Candidates

- 読み戻し（emo-present／emo-text）と、画像の符号化（MCP 側）の境。
- 今の見た目（画面の写し）と、指定 surface（画面外の合成）の 2 経路。

## Out of Boundary

- 拡大縮小・半透明の適用後の画像（SSP も「前」を返す）。
- 窓全体・デスクトップのスクリーンショット。

## Upstream / Downstream

- **Upstream**: `mcp-tool-entrances`。
- **Downstream**: 台本の dumpsurface／dumpballoon（未起票）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `text-align-shadow-canon`・`emo-text-canon-residue`（emo-text を触る α 後の spec。ウェーブが重なるなら干渉台帳で分ける）。

## Constraints

- 規模 M。GPU の読み戻しは UI スレッドで、サーバのスレッドを長く待たせない（待ちの上限は `mcp-tool-entrances`）。


---

## 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- 棚卸⑳では個別の再測定をしていない（`mcp-tool-entrances` が、各 spec の触るファイルを設計で固定する）。着手は `mcp-tool-entrances` の完了の後で、そのとき接触ファイルを照合する。
- emo-present と emo-text を読む＝文字まわりの spec と時期が重なるときは接触ファイルを照合する。

## 2026-10-03 C4 の候補（10-03 の再編（開発者「MCP は複合 spec なので早めに着手したい」））

- 段は「優先」。C3 に入れなかった理由: C3 の `balloon-font-file` と emo-text の境目が近い。

## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 規模: M（10〜14 タスク）。切らない。
- 前提の状態: `mcp-tool-entrances`（PR#223）は着地済み。前提は満たす。`emo-text-file-split`（C1）・`shell-balloon`（PR#227）も着地済み。
- 崩れた前提／古くなった位置:
  - ダミーの場所: アプリ本体側 `crates/areka/src/mcp/{dump_surface,dump_balloon}.rs` の `handle`（各 `NG:not implemented yet` の 1 文）と各 `_tests.rs`（書き換える）。プロトコル側 `crates/areka-mcp/src/tools/{dump_surface,dump_balloon}.rs` は `Args`（`scope: Option<i64>`・`surface: Option<i64>`・`ghost_name`）まで完成。画像を付ける口 `areka_mcp::tools::outcome::with_image(outcome, png_base64: String)` は在る（base64 済みの文字列を受ける＝符号化は本 spec）。
  - **emo-text の返事付きの変種は要らない見込み**: MCP の要求を汲む系（`mcp::drain`）は UI スレッドで走り、文字の層は World の中の `Rc<RefCell<TextLayerRuntime>>`（`emo2_boot/frame/wiring.rs` の結線状態の `runtime()`）にある。`TextLayerRuntime::surface_at(place)`／`surface(actor)` → `TextSurface::read_back()`（`areka-emo-text/src/surface.rs`・`pub`）を UI スレッドで直に呼べる。`TextMsg`（`sink.rs`）を広げる必要は無い。
  - 今の見た目の読み戻し: `EmoPresenter::read_back(target)`（`areka-emo-present/src/presenter/read.rs`）は合成メモの**原寸**の BGRA を返す（DPI の拡大 k に依らない＝SSP の「拡大の前」と合う）。結線状態の包み `read_back_target`（`emo2_boot/frame/wiring.rs`）は `#[cfg(test)]` のまま＝本番の口を 1 本開ける。`read_back` は幅・高さを返さない（大きさは別に取る）。
  - PNG の符号化: WIC の符号化器は `windows` の宣言済み機能で使える（`Win32_Graphics_Imaging_D2D` が `Win32_Graphics_Imaging` を含む・`areka-emo-atlas/src/decode/wic_arm.rs` が `CLSID_WICImagingFactory` を使用中）＝`Cargo.toml` 0 行。テスト側には依存なしの PNG 書き出し（`areka-emo-text/src/viewbox_draw_png_dump_tests.rs`・無圧縮）の前例もある。
- 触るファイル（並走の照合用）:
  - `crates/areka/src/mcp/{dump_surface,dump_balloon}.rs` と各 `_tests.rs`
  - **新規**の PNG と base64 の符号化（`mcp/mod.rs` を触らないよう、ツールのファイルの子モジュールにするか、別のクレートの新規ファイルに置く）
  - `crates/areka/src/emo2_boot/frame/wiring.rs`（`read_back_target` 相当の本番の口と、文字の層を引く口）
  - 指定 surface を画面外で合成するなら `crates/areka-emo-compose/src/lib.rs` の `compose` を呼ぶだけ（変えない）。変える必要が出たら `surface-element-nesting`（C3・emo-compose を触る）と同時に走らせない。
  - 触らない見込み: `areka-emo-present/src/presenter/read.rs`・`areka-emo-text/src/sink.rs`・`surface.rs`
- 議題（答えで作業が変わるものだけ）:
  - `dump_balloon` の「拡大の前」をどう作るか: 背景（emo-present）は原寸だが、文字の層の供給面は**物理 px＝`ceil(validrect 寸 × k)`**（`surface.rs` の `size` の注記）。k≠1 では 2 枚の大きさが合わない。⒜ 文字を縮めて重ねる ⒝ k=1 で画面外に描き直す ⒞ 拡大の後の大きさで返して差の一覧に書く、のどれか。
- 見つけた穴: `EmoPresenter::read_back` は未表示・メモから追い出された target で `error!` を出して `Err` を返す。MCP の呼び出しのたびに ERROR の行が出うる（`mcp-log-history` の着地後は error 種別の履歴にも積まれる）。MCP の経路では読む前に表示の有無を確かめて `NG:` で返し、`error!` を出さない形にする。

## 2026-10-04 ウェーブ C3-⑩（棚卸㉑）

- 段は「優先」。C3 は 11 本並走（`roadmap.md`「ウェーブ編成」の C3 の行が正本）。着手は最新の main から。
- 同じウェーブの約束: `crates/areka/src/mcp/mod.rs`・`handler.rs` と `areka-emo-compose` を触らない。本番の読み戻しの口は `emo2_boot/frame/wiring.rs` だけ（`frame/scale_text.rs`・`status_report.rs` は C3-③）。
