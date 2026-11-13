# Brief: pilot-windows-rs-next

## Problem
areka は Windows の API を `windows` クレート 0.62 で呼んでいる。開発者は windows-rs を 0.101.0 まで上げたい（2026-11-14）。きっかけは、開発者が上流へ出した縦書きの要望 [microsoft/windows-rs の issue 5369](https://github.com/microsoft/windows-rs/issues/5369) に、新しいクレート `windows-text` の縦書き対応（[PR 5379](https://github.com/microsoft/windows-rs/pull/5379)・2026-11-13 マージ）で答えが返ったことである。開発者は上流に「実アプリで各段を確かめる」と伝えている。

ところが、行き先は「`windows` の版を上げる」ではない。2026-11-14 の `/kiro-discovery` で開発者が決めたこと:

- **`windows` クレートは使わない**。`windows-core`・`windows-result` などを 0.101.0（公開の前は git 版）へ移し、API は `windows-bindgen` で作った束縛で呼ぶ（開発者「windows クレートは事実上使えなくなる」「windows api を呼ぶ方法が、windows クレートから、windows-bindgen を使って API をビルドする方法に変わるはず」）。
- **WinRT 側（Windows.UI.Composition・DispatcherQueue）も `windows-bindgen` で作る**。`windows-composition` へは乗り換えない（開発者「将来的に reactor をつかうことになると、windows-composition はリフテッドに固定されてしまい、areka プロジェクトで使ってる system と衝突するため」）。

この移行は、最初の見立て（1 本・16 タスク前後）では足りず、6〜8 本になる見込みである（開発者「ここの移行コストを甘く見て無いか？」）。束縛をどこに置くか、どの形で作るかで本体の切り方が変わるが、どれも試さないと決められない。方向と手順が怪しいので、本坑を掘る前に先進坑で確かめる。

## Current State
2026-11-14 に一次情報で確かめた事実（windows-rs の master の先頭は `520097a3`）。

**上流の版**

| クレート | crates.io の最新 | master |
|---|---|---|
| `windows` | 0.62.2（2025-10-06 のまま） | 0.100.0（未公開） |
| `windows-core`・`windows-numerics`・`windows-implement`・`windows-interface` | 0.100.0（2026-09-03） | 0.101.0（未公開） |
| `windows-bindgen` | 0.100.0 | 0.101 系 |
| `windows-link` | 0.100.0 | 0.100.1 |
| `windows-text` | 0.0.0（名前の確保だけ） | 0.101.0（未公開・縦書き入り） |

- 0.101.0 の公開日を述べた一次情報は無い。crates.io の `windows` 0.62.2 は `windows-core` 0.62 に固定で、core だけを先に上げる道は無い。
- 上流の `docs/readme.md`: 再利用するライブラリは `windows`・`windows-sys` を避け、絞ったクレートと `windows-bindgen` の自分用の束縛を使う。`crates/libs/bindgen/readme.md`: 公開するライブラリは束縛を別の道具で生成してコミットする。フィルターは API を 1 つずつ名指しし、インターフェースのメソッドも明示で選ぶ。出力は既定（`windows-core` を使う型つき）・`.sys()`（`windows-link` だけ）・`.minimal()`（手書きの包み向け）の 3 つで、`#[implement]` 用のトレイトは `.implement(名前)` で選んだものだけ出る。
- 束縛の形は今と大きく違う: `HWND` は生ポインタの別名（`Send` でない）・列挙は `i32`／`u32`・戻り値の無い COM メソッドは `HRESULT` を返す・スライスの引数は「ポインタ＋長さ」・`#[implement]` の `_Impl` トレイトは `HRESULT` を返す（PR 5299）・`windows-numerics` のフィールドは小文字（`x`・`m11`）・`HSTRING` の `Display` は無い。
- `windows-composition` の機能 `system`（Windows.UI.Composition）と `reactor`（リフテッド）は排他で、両方立つと `compile_error!`。`system` は `windows-window` を引く。
- `windows-text`（master）: 依存は `windows-core` と `windows-numerics` だけで、自前の束縛を持つ。`TextFormat`・`TextLayout`・`WritingMode::VerticalRightToLeft`・`hit_test_point`・`range_bounds`・`glyph_runs()`・`decorations()`・`set_font_features`・`FontCollection::from_data`・`FontFallback` が在る。**無いもの**: 範囲ごとの色（`SetDrawingEffect`）・文中のインラインオブジェクト・字送り（`SetCharacterSpacing`）・縦中横・文字単位の折り返し。`TextLayout::new` は失敗で panic する。位置は UTF-8 のバイト位置。生の COM へは `as_interface::<T>()`／`to_interface::<T>()` で出入りでき、`T` は `windows-core` 0.101 の `Interface`（`as_interface` は 0.100.0 に無い・PR 5331）。`IDWriteFactory` は取り出せない（プロセスで共有の 1 個を中で持つ）。

**areka の側**

- `Cargo.lock` の `windows` まわりは 1 系列だけ（`windows` 0.62.2・`windows-core` 0.62.2・`windows-numerics` 0.3.1・`windows-implement` 0.60.2・`windows-interface` 0.59.3）。
- `windows::` を綴る行は約 1,300 行（`std::os::windows::` の 49 行を含む数え）。多い順に `wintf` 721 行／206 ファイル・`areka` 198 行・`areka-emo-text` 107 行・`pilot` 63 行・`areka-placement` 51 行・`shiori-host32-host` 50 行。`windows_numerics::` は 61 行。
- ワークスペースの機能は 27 個（Direct2D・Direct3D11・DirectComposition・DirectWrite・Dwm・Dxgi・Gdi・WIC・COM・Shell・WinRT の `UI_Composition`・`UI_Composition_Desktop`・`System`・`Foundation`・`Graphics_DirectX` ほか）に、クレートごとの上乗せが在る（WinHTTP・CNG・ダイアログなど）。
- `#[interface]` は `shiori-abi` の 3 つ（`IShiori`・`IShioriHost`・`IShioriFactory`）。`#[implement]` は SHIORI のインターフェースに 25 か所前後（`shiori-abi`・`areka`・`areka-ghost`・`shiori4-testdll`）と、`IDWriteTextRenderer1` に 2 か所（`crates/areka-emo-text/src/viewbox_draw_underline.rs`・`draw_line_store_tests.rs`）。
- DirectWrite: 縦書きは TextFormat の読みの向き TOP_TO_BOTTOM＋流れの向き、`SetLineSpacing`、`IDWriteTypography` の縦書きの字形の指定で組む（`crates/areka-emo-text/src/draw.rs`）。列は 1 本ずつの TextLayout で、折り返し・列の配置・選択肢の当たりは areka の自前。描くのは `ID2D1DeviceContext::DrawTextLayout`（カラーフォントの指定つき）で、色は `SetDrawingEffect`。下線は自前の `IDWriteTextRenderer1` で位置だけを集める。書体の名前は `IDWriteGdiInterop` で引き直す（`draw_catalog.rs`）。
- 合成: Windows.UI.Composition（`crates/wintf/src/com/wuc.rs`）。描画先は D3D11 のテクスチャを包んだ `ID2D1Bitmap1` で、スワップチェーンを SpriteVisual に載せる。
- よその依存: `sysinfo` 0.38.4（`human-panic` 経由）が `windows` を 0.63 未満に固定している。古い系列が木に並ぶだけで、`deny.toml` の `multiple-versions` は `warn`。
- 公開の制約: `wintf`・`dola`・（取り込みの後は）`wintf-winmsg-executor` は crates.io へ出す。crates.io へ出すクレートは git の依存を持てない。`deny.toml` は `unknown-git = "deny"` で、許しているのは `image-webp` の 1 件だけ。

## Desired Outcome
下の問いのそれぞれに、go／違う／直す の判定と学びが付いた一次記録（README）が在り、開発者がそれを見て移行の本体へ進むかを決められる。あわせて次が出ている。

- **移行の本体を何本にどう刻むかの案**（束縛を作る道具・クレートの群ごとの移行・`windows` 0.62 を外す締め・`windows-text` の採用）と、それぞれの規模の実数に基づく見立て。
- **上流へ返す文案**（`windows-text` の足りない所・実アプリで確かめた結果）。出すのは開発者の承認の後。

**確かめる問い**

1. **C の関数だけのクレート**: `wintf-winmsg-executor` の写しを `.sys()` の束縛へ移して例が動くか（x64・i686）。フィルターの行数と、移すのにかかった手間。
2. **束縛の置き場所**: `wintf` と `areka-emo-text` の間で `ID2D1DeviceContext` などの COM の型を受け渡す形を、束縛をクレートごとに持つ・`wintf` が束縛を公開する・共有の束縛のクレートを置く、のどれで組むか。`windows-bindgen` に、ほかのクレートの束縛の型を参照する口が在るか。
3. **束縛の形**: 既定（型つき）か、`.minimal()`＋`wintf` の `com/` の手書きの包みか。
4. **WinRT**: `windows-bindgen` の束縛で、今の `com/wuc.rs` の形（Compositor・DesktopWindowTarget・描画面と SpriteVisual のつなぎ・相互運用のインターフェース・DispatcherQueue）がそのまま通るか。
5. **`HWND`**: 生ポインタの別名を bevy の Component に持つための包み方。
6. **COM の実装**: `windows-core` 0.101 の `#[interface]`・`#[implement]` で、SHIORI の 1 組と `IDWriteTextRenderer1` が組めるか。
7. **`windows-text` の縦書き**: 今の areka の 1 列（開き括弧`「`・小書きかな・句読点・絵文字を含む）と画素で比べて同じか。`set_font_features` で今の縦書きの字形の指定を再現できるか。`decorations()` で下線の位置が取れるか。範囲ごとの色を `as_interface` 経由の `SetDrawingEffect` で付けられるか。UTF-8 の位置と panic する `new` をどう扱うか。
8. **リンク**: `windows-link` の束縛で i686 と arm64 が通るか。
9. **規模**: 使っている API の全数（関数・型・定数・メソッド）の数え方と実数。
10. **git の依存**: 公開の前の 0.101 の系列を、本線の `Cargo.lock` に入れてよいか（`deny.toml` の許し・公開の道具の、ネットを使わない包みの段が通るか）。

## Approach
二坑モデル（`.kiro/steering/two-tunnel.md`）の先進坑として掘る。成果物はコードでなく知見である。

- 上流は master の決まったコミットを git で引く（引いたコミット番号を README に書く）。
- 小さく縦に通す。全クレートを移そうとしない。問い 1 は `wintf-winmsg-executor` の写し、問い 2〜5 は DirectWrite・Direct2D・WUC を 1 本だけ通す最小の例、問い 6 は SHIORI の 1 組、問い 7 は縦書きの 1 列で確かめる。
- 出荷するクレート（`crates/wintf` ほか）は書き換えない。試すのは写しの上か、知見のクレート `crates/pilot` の `examples/pilot-windows-rs-next/` の中だけ。
- 一次記録は `crates/pilot/examples/pilot-windows-rs-next/README.md`（動機・概要・検証結果の 3 幕）。

## Scope
- **In**: 上の 10 の問いの確かめ・一次記録・移行の本体の刻み方の案・上流へ返す文案。
- **Out**:
  - 出荷するクレートの書き換え（移行の本体は別に起票する）。
  - `windows-composition`・`windows-canvas`・`windows-reactor` の採用（裁定済み・reactor 待ちの spec は別）。
  - `windows-text` の本採用（移行の本体の後の `emo-text-windows-text`）。
  - `wintf` の label・typewriter の `windows-text` 化。
  - 上流への投稿そのもの（文案まで。出すのは開発者の承認の後）。

## Boundary Candidates
- C の関数の束縛（`.sys()`）と、束縛を作る手順
- COM と WinRT の束縛の置き場所と形
- `windows-text` の縦書きの確かめ
- 規模の数えと、本体の刻み方の案

## Out of Boundary
- 束縛を作る道具の完成品と、全体テストの見張りの段（本体の 1 本目が持つ）
- 版の段と着手の席の組み直し（次の棚卸）
- リリースの手順の変更（git の依存と公開の兼ね合いは、この先進坑の結果を見て開発者が決める）

## Upstream / Downstream
- **Upstream**: `wintf-winmsg-executor-import`（取り込んだクレートを問い 1 の的にする）。上流の windows-rs の master（未公開で、公開までに API が動きうる）。
- **Downstream**（どれもこの先進坑の go を前提にする＝`_Depends(confirmed): pilot-windows-rs-next`・brief は go の後に `/kiro-discovery` で書く・切り方は仮）:
  1. 束縛を作る道具と、食い違いを全体テストで止める段
  2. `wintf-winmsg-executor` と `dola`（C の関数だけ）
  3. COM の型をほかのクレートとやり取りしないクレート（`areka-update`・`shiori-host32-*` など）
  4. SHIORI の `#[interface]`・`#[implement]`（`shiori-abi` ほか）
  5. `wintf`
  6. `areka-emo-*`・`areka-placement`・`areka`
  7. `windows` 0.62 をワークスペースから外す締め
  8. `emo-text-windows-text`（`areka-emo-text` の DirectWrite を呼ぶ下回りを `windows-text` に任せる。折り返し・列の配置・選択肢の当たりは自前のまま）
- `windows-text` の口を使える後続: `balloon-font-file`（`FontCollection::from_data`）・`text-reveal-fade`（`glyph_runs()`）・`text-typesetting`。
- windows-reactor 待ちの spec 群は、移行の本体が上流になる（reactor 自体の待ちは別に残る）。

## Existing Spec Touchpoints
- **Extends**: 無し。
- **Adjacent**: v0.2.0 の `wintf` の作り直しの spec 群（この先進坑は `crates/wintf` を書き換えないので重ならない）。`crates/pilot` に触るほかの先進坑。

## Constraints
- **二坑モデルの決まりに従う**: コードは `crates/pilot/examples/pilot-windows-rs-next/` に隔離し、出荷するクレートが先進坑のコードに頼らない。先進坑のコードを本坑へ写して使わない。go の判定は開発者。
- **git の依存を本線に入れられないとき**（問い 10 が「違う」のとき）は、この spec に限り、本線へ入れるのを README と刻み方の案だけにし、コードは枝に残す。この例外は自分の要件・設計・タスクに書き、共通の文書（`two-tunnel.md`・`workflow.md`）は直さない。
- **裁定済みのことを議題に戻さない**: `windows` クレートを使わない・WinRT は `windows-bindgen`・`windows-composition` は使わない。
- **外部クレートはそのまま使う**（フォークも `[patch]` もしない）。上流の master を決まったコミットで引くのは、未公開の版をそのまま使う形で、書き換えではない。足りない所は公開の型で自分の側に組む。
- **上流は動く**: `windows-text` は 2026-11-06 にできたクレートで、0.101 は未公開。引いたコミット番号と確かめた日を必ず書き、公開の後に結果が変わりうることを README に書く。
- **GPU の確かめは実の GPU で**。実機の根・検体・一時フォルダは作業木の `target` の下だけ。
- **上流への投稿は開発者の名で外へ出る**。文案を作り、承認の後に出す。
- **`windows-core` は 1 系列にそろえる決まり**（`windows` と同じ版で使う）は、今の本線の話である。先進坑の例の中では新旧が並ぶ。本線で新旧を並べてよいか（段階の移行）は、この先進坑の結果として開発者に示す。
- **規模**: M（10〜14）。問いが多いので、要件の段で問いを削るか 2 本に分けるかを決めてよい。
