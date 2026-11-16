# ギャップ分析: pilot-windows-rs-next

- 分析した日: 2026-11-16
- 分析の型: 既存のコードの上に新しい物を足す型（brownfield）。先進坑なので「既存の部品を広げる」でなく、本線の外に独立の置き場を建て、本線のクレートを写すか借りる。
- 確かめた範囲: `requirements.md`（要件 1〜15・仮定 1〜8）・`brief.md`・steering（`tech.md`・`structure.md` の pilot の節・`two-tunnel.md`・`roadmap.md` の「windows-rs の移行」の節）・本線のコード（下の各節）・上流の windows-rs の master（GitHub の API とソース）。
- 要件は未承認（`spec.json` の `approvals.requirements.approved = false`）。本書は要件の討議の材料で、決めは下さない。

---

## 1. 要約

- **置き場は要件 1 のとおり建てられる**。根の `members = ["crates/*"]` は 1 段だけで、`crates/pilot/examples/pilot-windows-rs-next/` に独立の `[workspace]` を置いても本線のビルドに乗らない。ただし見張りの試験は `crates/` の下を**深さに関係なく**読む（`.rs` も `Cargo.toml` も）ので、置き場のコードと manifest にも本線と同じ禁止が掛かる（2.2 の表）。
- **上流の事実が brief から 3 点ずれた**: ⑴ master の先頭は `520097a3` から `f37e2f90`（2026-11-14 20:13Z）へ進んだ。⑵ `windows-bindgen` 0.100.0 と master には、ほかのクレートの束縛の型を参照させる公開の口が**無い**（0.62 の `--reference` は消えている）。⑶ `HWND` が生ポインタの別名になるのは `.sys()` と `.minimal()` だけで、既定の形では今と同じ newtype のまま。
- **写す量は 1,500 行前後に収まらない見込み**: 写しの元の実数は合わせて約 1,800 行（テストを含む）で、問い 10 の 2 つ目の写しを足すと約 2,900 行になる。
- **問い 9（規模の数え）は grep では数えられない**: `use windows::…::*` の glob の取り込みが本線に 110 か所あり、どの名前を使っているかが字面に出ない。コンパイラか rust-analyzer に頼る数え方が要る。
- 規模の見立ては L（1〜2 週）・危険は高い（上流が未公開で動く・GPU と画面が要る）。M（10〜14 タスク）に収めるなら問いを絞るか 2 本に分ける判断が要る（仮定 1）。

---

## 2. 今の状態（コードベースの調べ）

### 2.1 置き場とワークスペース

| 事実 | 確かめた所 | 本仕様への意味 |
|---|---|---|
| 根のワークスペースの `members` は `crates/*` の 1 段だけ。`exclude` は無い | 根の `Cargo.toml` 1〜5 行 | 置き場の下の `Cargo.toml` は本線の一員にならない。`exclude` も要らない（brief のとおり） |
| `crates/pilot` は空の lib＋`examples/` の構造。例の自動発見は `examples/<名前>/main.rs` だけ | `crates/pilot/Cargo.toml`・`examples/` の一覧 | 置き場の直下に `main.rs` を置かなければ、`crates/pilot` の例として組まれない（要件 1.5） |
| 既存の先進坑の例は 6 つ（`_template`・`pilot-balloon-asset-swap`・`pilot-clickthrough-alpha-toggle`・`pilot-dropfiles-on-wuc-window`・`shiori-host-32`・`wintf-winmsg-executor`）。どれも `crates/pilot` の例で、独立のワークスペースは前例が無い | `crates/pilot/examples/` | 置き場の形は本仕様が初めて。README の 3 幕のひな形は `_template/README.md` を使える |
| 根に `.cargo/config.toml` は無い | 作業木の根 | 置き場の中に `.cargo/config.toml` を置けば、そこで打つ cargo の `target-dir` を作業木の `target` の下へ向けられる（要件 15.2）。本線へは効かない |
| `.gitignore` の `target` はどの深さにも効く | 根の `.gitignore` | 置き場の中に `target` ができてもコミットされない |
| crates.io の公開の確かめ（`tools/crates-io.ps1`）は `cargo metadata --no-deps` で本線のワークスペースだけを見る | `tools/crates-io.ps1` 246 行 | 置き場の manifest に `publish` の行が無くても全体テストの公開の段は赤にならない（ただし書いておくのが無難） |

### 2.2 置き場に及ぶ見張りの試験（要件 1.7）

走査器 `crates/log-capture-kit/tests/workspace_scan/mod.rs` は `crates/` の下を再帰で読み、除くのはフォルダ名が `target`・`vendors`・`.git` のものだけ（48 行の `EXCLUDED_DIRS`）。`.rs` も `Cargo.toml` も同じ規則で集める（89 行・476 行・497〜516 行）。よって置き場の中身にも次がそのまま掛かる。

| 見張り | 置き場で守ること | 赤になる例 |
|---|---|---|
| `file_length_guard_test.rs`（1 ファイル 1,000 行） | `.rs` をどれも 1,000 行以下に。`windows-bindgen` の出力を置き場にコミットするなら分ける | 生成した `bindings.rs` を 1 本でコミットする |
| `temp_path_guard_test.rs` | `std::env::temp_dir` を呼ばない（許可表に無いので 1 件でも赤） | 差の絵の一時ファイルを OS の一時フォルダへ書く（要件 15.2 とも衝突） |
| `with_default_guard_test.rs` ⑴⑵ | tracing の捕捉先を差す 4 語（`with_default(`・`set_default(` ほか・走査語は同ファイル 62〜68 行）を綴らない | 例の中で購読者を手で据える書き方。既存の先進坑の例は `tracing_subscriber::fmt()…init()` で据えていて当たらない |
| `with_default_guard_test.rs` ⑶⑷ | `Cargo.toml` の本番の依存に `log-capture-kit` を書かない・`tracing-subscriber` の機能 `env-filter` を宣言しない（宣言してよいのは `wintf` だけ） | 置き場の manifest で `features = ["env-filter"]` を書く |
| `sample_path_guard_test.rs` | 検体のパスを自分で組まない | 問い 7 で検体のバルーンを使うときに `vendors/sample_ghost` を綴る |
| `crates/areka/src/runtime/frame_attach_tests.rs`（`state.actors()` の走査点） | `crates/` の下を再帰で読む（`target` だけ除く）。本番の `.rs` で `state.actors()` を書かない | 問い 7 で `areka-emo-text` を借りて同じ走査を書く |
| `crates/areka/src/session_end_sync_send_tests.rs` | 読むのは `crates/*/src/**/*.rs` だけ。置き場は読まない | 当たらない（brief の申し送りのとおり） |

フォルダ名 `target` は深さに関係なく走査から外れるので、`windows-bindgen` の出力を `build.rs` で `OUT_DIR` に作る形なら、置き場の中の `target` でも作業木の `target` でも見張りに当たらない。

### 2.3 写しの元の実数（2026-11-16・本作業木）

| 問い | 写しの元 | 行数 | 備考 |
|---|---|---|---|
| 1 | `crates/wintf-winmsg-executor/src/`（`lib.rs` 592・`util/window.rs` 373・`util/msg_filter_hook.rs` 64・`util/mod.rs` 7） | 1,036 | `lib.rs` の 271 行目から後ろは `#[cfg(test)] mod test`（約 320 行）。brief の「約 920 行」とは数え方が違う |
| 1 | 例 `basic.rs` 36・`threads.rs` 37 | 73 | |
| 3 | `crates/wintf/src/com/dwrite.rs` | 154 | `use windows::Win32::Graphics::DirectWrite::*;`（glob） |
| 3 | Direct2D の一部＝`crates/wintf/src/com/d2d/mod.rs` | 158 | `com/` の下に `d2d.rs` は無く、`d2d/mod.rs` が当たる |
| 4 | `crates/wintf/src/com/wuc.rs` | 254 | 使う型は `DispatcherQueueController`・`CompositionGraphicsDevice`・`DesktopWindowTarget`・`ICompositionSurface`・`ICompositorInterop`・`ICompositorDesktopInterop`・`ICompositionDrawingSurfaceInterop`・`CreateDispatcherQueueController` ほか |
| 6 | `crates/shiori-abi/src/interface.rs` の 1 組 | 約 100〜150 | 全体 476 行。`#[interface]` 3 つ（52・84・125 行）、試験の中の `#[implement]` 3 つ |
| 6 | `crates/areka-emo-text/src/viewbox_draw_underline.rs`（`IDWriteTextRenderer1`） | 402 | 写すか新しく書くかは要件 7.2 が決めていない。実装は `IDWritePixelSnapping_Impl`・`IDWriteTextRenderer_Impl`・`IDWriteTextRenderer1_Impl` の 3 段 |
| 合計（問い 1・3・4・6・テスト込み） | | 約 1,800〜1,850 | 要件 1.8 の「1,500 行前後」を超える。問い 10 の 2 つ目の写し（要件 11.1・約 1,100 行）を数えると約 2,900 行 |

本線の使い方の数（`crates/pilot` を除く・本作業木）:

- `windows::` を綴る行は 1,264 行（`std::os::windows::` を除く）。ファイルの数は `wintf` 214・`areka` 88・`areka-emo-text` 34・`areka-placement` 31・`shiori-host32-host` 9 ほか 22 クレート。
- `use windows::…::*` の glob の取り込みは 110 か所。
- `#[implement(` は 47 か所（試験を含む。brief の「SHIORI に 25 か所前後＋`IDWriteTextRenderer1` に 2 か所」は本番の数）、`#[interface(` は 4 か所。
- `HWND` の字面は `wintf` だけで 992 回。

### 2.4 既存の型と書き方（問い 5・要件 12 に効くもの）

- **`HWND` を Component に持つ書き方は既に在る**: `windows` 0.62 の `HWND` も生ポインタを包む newtype で `Send` でないので、`wintf` は持ち主の構造体に `unsafe impl Send/Sync` を付けている（`ecs/window/window_handle.rs` の `WindowHandle`・`ecs/window/components.rs` の `Window`・`ecs/drag/context.rs`・`ecs/window/window_pos/mod.rs` ほか・`wintf` 全体で 35 か所）。問い 5 で新しく要るのは「包み方」より、`HWND(ptr)` の作り方・`.0`・`is_invalid()`・`Default`・`PartialEq`/`Hash` が束縛の形ごとにどう変わるかの対応（3.3）。
- **COM の型の `==` に頼る所は 2 か所**: `crates/wintf/src/ecs/graphics/composite.rs` 183〜185 行の `same_root`（`Option<&ID2D1CommandList>` の `==`）と、`crates/wintf/src/ecs/graphics/command_list.rs` の `#[derive(Component, Debug, Clone, PartialEq)] struct GraphicsCommandList`（導出の `PartialEq` が中の `ID2D1CommandList` の `==` に頼る）。同じファイルに `unsafe impl Send/Sync`（D2D の多スレッド工場が根拠）。
- **今の areka の縦書きを画素で読み戻す道は既に在る**: `crates/areka-emo-text/tests/decoration_readback/mod.rs` が、headless の GPU World・`present_frame` の本番の経路・供給面の BGRA の読み戻し・墨の判定を持つ（縦書きは `vertical.rs` の `shoot_column`）。公開の API（`areka_emo_text::actor`・`state::TextLayerConfig`・`areka_parsers::balloon`・`areka_sakura::contract`）だけで組んであるので、置き場から path で借りて同じ組み方を新しく書ける（試験の補助そのものは `tests/` の中で外から使えない）。
- **`VERTICAL_FEATURES` は今も 10 個**: `crates/areka-emo-text/src/draw.rs` 456〜467 行（`ccmp`・`locl`・`mark`・`mkmk`・`rlig`・`calt`・`clig`・`liga`・`kern`・`rclt`）。`draw_line_store_tests.rs` 489 行が `vpal`・`vert`・`vkrn` を含まないことを確かめている。
- **i686 で気を付ける所**: `wintf-winmsg-executor` の `util/window.rs` は `GetWindowLongPtrW`／`SetWindowLongPtrW` を 4 か所で呼ぶ（221・246・251・275 行）。32bit の Windows にはこの関数の書き出しが無く、`windows` 0.62 は 32bit で `…LongW` へ別名を張っている。`.sys()` の出力が同じ別名を出すかは未確認（Research Needed）。

### 2.5 手元の道具（要件 9・10 に効くもの）

| 事実 | 確かめ方 |
|---|---|
| Rust 1.100.0（2026-11-09）・cargo 1.100.0 | `rustc --version` |
| 入っているターゲット: `x86_64-pc-windows-msvc`・`i686-pc-windows-msvc`・`aarch64-pc-windows-msvc`（ほかに `aarch64-unknown-linux-gnu`・wasm 2 つ） | `rustup target list --installed` |
| VS 18 Community に `VC.Tools.ARM64` と `VC.Tools.x86.x64` が在る | `vswhere.exe -requires …` |
| 機械は x64（Intel Core Ultra 9 185H）。arm64 の機械は無い | `Win32_Processor` |
| nightly のツールチェーンと `rust-analyzer` が在る | `rustup toolchain list`・`rustup component list --installed` |

i686 と arm64 のビルドの準備は済んでいる。arm64 は「組めてリンクが通るまで」が手元の上限（仮定 5 のとおり）。クロスのビルドは PowerShell から打つ（Git Bash では `link.exe` が隠れる・`tech.md`）。

---

## 3. 上流の事実（2026-11-16 に GitHub で確かめた）

### 3.1 版と先頭のコミット

| 項目 | 値 |
|---|---|
| master の先頭 | `f37e2f9046cf9710fe00285436c4d22ad6053d72`（2026-11-14T20:13:16Z「Add submitted-frame input support (#5384)」）。brief の `520097a3` より先へ進んでいる |
| `windows-core`・`windows-text`・`windows-bindgen`・`windows-numerics`・`windows-implement`・`windows-interface` | 0.101.0（master・未公開） |
| `windows-link`・`windows-result`・`windows-strings` | 0.100.1（master） |
| `windows`（クレート） | 0.100.0（master）・edition 2024・`rust-version = "1.95"` |
| `windows-text` の依存 | `windows-core` と `windows-numerics` だけ（どちらも上流のワークスペースの依存）。ソースに自前の `bindings.rs` を持つ |
| リポジトリの大きさ | 約 1.4 GB（GitHub の `size`）。cargo の git の依存は丸ごと取り寄せ、置き場は `CARGO_HOME` の下（作業木の外） |

### 3.2 `windows-bindgen` にほかのクレートの型を参照させる口（要件 4.3 の答えの候補）

- **0.100.0（crates.io の最新）と master に、公開の口は無い**。
  - master の `crates/libs/bindgen/src/cli.rs` が読む旗は `--in`・`--out`・`--filter`・`--filter-file`・`--rustfmt`・`--derive`・`--flat`・`--package`・`--sys`・`--extern`・`--minimal`・`--dead-code`・`--implement`・`--compose` だけで、知らない旗は `panic!("invalid option …")` で止まる。
  - master の `src/lib.rs` の builder の公開の方法にも参照の口は無い。参照の表（`ReferenceStage`）を作るのは内部の `prepend_default_refs` 1 か所だけで、既定の形（`.sys()` でない）のときに、上流の兄弟のクレート（`windows_future`・`windows_collections`・`windows_reference`・`windows_time`・`windows_numerics`）の型を暗に参照させる。利用者のクレートは足せない。
  - docs.rs の 0.100.0 の `lib.rs` にも `--reference` の字は無い。
- 0.61・0.62 の `windows-bindgen` には `--reference` が在った（docs.rs のソース）。どの版で消えたかは未確認（Research Needed・本仕様の答えには要らない）。
- **上流そのものが「クレートごとに束縛を持つ」形を取っている**: `windows-text` は自前の `bindings.rs` を持ち、生の COM へは `as_interface::<T>()`／`to_interface::<T>()`（`T` は `windows-core` 0.101 の `Interface`）で出入りする。型の違う 2 つの束縛を、同じ `windows-core` の上で `cast`（QueryInterface）でつなぐ形の実例になる。

### 3.3 束縛の形と `HWND`（要件 6・12 に効く）

master の `src/lib.rs` の `Style`:

- `emit_bare_typedef()` は `is_sys() || is_minimal()`。つまり **`HWND` などの取っ手が生ポインタの別名になるのは `.sys()` と `.minimal()` のときだけ**で、既定の形では newtype（今の `windows` 0.62 と同じ形）で出る。
- `.minimal()` は class の包み・継いだメソッドの転送・取っ手の便利な口を出さない（継いだメソッドは持ち主のインターフェースへ `cast` して呼ぶ）。入力の文字列は `&str`、戻りの文字列は `String`。
- `.sys()` は `Copy`/`Clone` のほかの導出を出さず、`windows-core` の trait も出さない。

要件 6.1（「生ポインタの別名で `Send` でない」）と要件 12.2 の `HWND` の行は、`.sys()`／`.minimal()` を選んだときだけ当たる。既定の形を選ぶと、`HWND` の書き換えはほぼ要らない。どの形を選ぶかは問い 3 の答えなので、問い 5 の確かめは問い 3 の形の両方で見る必要がある。

### 3.4 COM の型の `==`（要件 12.4・12.5）

master の `crates/libs/core/src/unknown.rs` の `impl PartialEq for IUnknown` は、番地が同じならそれで等しく、違えば両方を `IUnknown` へ `cast` して比べる（COM の同一性を保つ）。`Eq` も在る。ただし、**束縛の中のインターフェースの型（例 `ID2D1CommandList`）が `PartialEq` を導出するかは形によって違いうる**（`.sys()` は `Copy`/`Clone` のほかを導出しない）。生成した型の実物で確かめる必要がある（Research Needed）。

### 3.5 未確認の上流の事実（Research Needed）

1. `#[interface]` マクロで作るインターフェース（SHIORI の 3 つ）の `_Impl` トレイトも `HRESULT` を返す形に変わったか（PR 5299 は bindgen の出力の話か、マクロも含むか）。
2. `.sys()` の出力が 32bit で `SetWindowLongPtrW` → `SetWindowLongW` の別名を出すか（出さないなら写しの側で `cfg(target_pointer_width)` の分けが要る）。
3. `.implement("IDWriteTextRenderer1")` が継いだ `IDWriteTextRenderer`・`IDWritePixelSnapping` の `_Impl` も出すか、3 つとも名指しが要るか。
4. git の依存で上流のワークスペースの一員（`windows-text` など）を引いたとき、上流のワークスペースの `members` に在る試験・道具のクレートが解決を止めないか（brief の先行検証は「要るフォルダだけ取り寄せ」の形で、git の依存の形では未確認）。
5. 上流の `rust-version = "1.95"` と edition 2024 は手元の 1.100.0 で問題ない見込み。

---

## 4. 要件ごとの資産の対応表

印: **Missing**＝今は無く新しく作る／**Unknown**＝確かめないと分からない／**Constraint**＝既存の決まりから来る縛り。

| 要件 | 使える既存の資産 | ギャップ |
|---|---|---|
| 1 置き場と隔離 | `crates/pilot` の構造・`_template/README.md`・根の `members` の 1 段 | **Missing**: 独立の `[workspace]` の前例が無い。**Constraint**: 2.2 の見張りが置き場の `.rs` と `Cargo.toml` に掛かる。**Constraint**: 写す量の上限（1.8）が実数（約 1,800 行・問い 10 込みで約 2,900 行）と合わない |
| 2 上流の版の固定 | brief の版の表 | **Unknown**: どのコミットを引くか（master は `f37e2f90` へ進んだ）。**Unknown**: git の依存で解決が通るか（3.5 の 4）。**Constraint**: 約 1.4 GB の取り寄せが `CARGO_HOME`（作業木の外）に入る |
| 3 問い 1 | `crates/wintf-winmsg-executor/`（1,109 行）・例 2 本 | **Missing**: `.sys()` の束縛とフィルター。**Unknown**: i686 の `…LongPtrW`（3.5 の 2）。**Constraint**: `lib.rs` の `#[cfg(test)]` の約 320 行を写すかで量が変わる |
| 4 問い 2・3 | `com/dwrite.rs`・`com/d2d/mod.rs`・`windows-text` の `as_interface` の前例 | **Missing**: 役を演じる 2 クレートと 3 つの置き場の案。**Constraint**: 参照の口が無い（3.2）ので、案ごとに「束縛の重なり」と「`cast` でつなぐ手当て」が要る形が決まる。**Constraint**: `dwrite.rs` は glob の取り込みで、使う名前を数え直す必要がある |
| 5 問い 4 | `com/wuc.rs`（254 行）・`pilot-dropfiles-on-wuc-window` など WUC の窓を出す先進坑の前例 | **Missing**: WinRT の束縛（`Windows.UI.Composition`・`Windows.System.DispatcherQueueController`・`Win32.System.WinRT.Composition` の相互運用）。**Unknown**: 既定と `.minimal()` で class の口（`Compositor::new` など）がどう出るか |
| 6 問い 5 | `WindowHandle` ほか `unsafe impl Send/Sync` の 35 か所 | **Constraint**: 「生ポインタの別名」は `.sys()`／`.minimal()` のときだけ（3.3）。**Missing**: 形ごとの書き換えの実例 |
| 7 問い 6 | `shiori-abi/src/interface.rs`・`viewbox_draw_underline.rs` | **Unknown**: マクロの `_Impl` の戻り値（3.5 の 1）・`.implement` の継ぎ（3.5 の 3） |
| 8 問い 7 | `decoration_readback/` の読み戻しの組み方・`VERTICAL_FEATURES`（10 個）・上流の `text-vertical` のサンプル | **Missing**: 両側を同じ大きさ・同じ書体・同じ位置で描いて比べる段取り。**Constraint**: 新旧の COM の型はつなげない（要件 8.2）。**Constraint**: 画面の操作は排他テストの机（要件 15.3）。**Unknown**: `TextLayout` を自前の `ID2D1DeviceContext::DrawTextLayout` で描くとき、`windows-text` の中の `IDWriteFactory`（取り出せない）とこちらの D2D の装置が組めるか |
| 9 問い 8 | 入っている i686・aarch64 のターゲット・VS の ARM64 の道具 | **Constraint**: arm64 の機械が無い＝リンクまで。**Unknown**: `windows-link` の取り込みの指定が i686 の `stdcall` の飾りで通るか |
| 10 問い 9 | grep の数（1,264 行・glob 110 か所）・`rust-analyzer`・nightly | **Missing**: だれが走らせても同じ数が出る数え方。**Constraint**: glob の取り込みのため grep では関数・型・定数・メソッドに分けて数えられない |
| 11 問い 10 | 問い 1 の写し・対応表 | **Constraint**: 2 つ目の写しが要件 1.8 の量を押し上げる。**Missing**: わざと入れる取りこぼし 3 種と、それを lint で見る手順（clippy の設定） |
| 12 対応表 | brief の書き換えの一覧・申し送り（`same_root`・`GraphicsCommandList`） | **Constraint**: `HWND`・`==` の行は束縛の形で答えが変わる（3.3・3.4）。行ごとに「どの形のときか」を書く欄が要る |
| 13 一次記録 | `_template/README.md`・既存の先進坑の README | **Missing**: 本文。段階の移行の結果（要件 13.5）は brief の先行検証（`cargo tree` で並ぶ）を、置き場の中の実物で確かめ直す形になる |
| 14 上流へ返す文案 | brief の「無いもの」の一覧 | **Missing**: 文案 |
| 15 実機の作法 | `tech.md` の GPU の規律・排他テストの机（`areka-impl-watch`）・`temp-path-kit` の決まり | **Constraint**: 置き場は本線の `temp-path-kit` を使えない形（借りれば使える）・`std::env::temp_dir` は見張りが禁止。**Constraint**: 窓を出す例は上限時間の内で自分で終わる |

---

## 5. 進め方の案

先進坑なので「既存を広げる／新しく作る」は、置き場の組み方と、束縛の作り方の 2 つの軸で並べる。

### 5.1 置き場の組み方

#### 案 A: 本線の `crates/pilot` の例に足す（既存を広げる）
- 本線の `crates/pilot` は既に `windows-core` 0.62 を持つので、`windows_core` の名前で 0.101 を持てない。`#[implement]`・`#[interface]` と bindgen の出力は `windows_core::` を名指しする。
- 根の `Cargo.lock` と `deny.toml`（`unknown-git = "deny"`）に git の依存が入り、要件 1.3 に反する。
- ✅ 新しい形を作らない ❌ 組めない・要件に反する。**採れない**（brief で既に外した案。比べのために置く）。

#### 案 B: 独立のワークスペースに、写しも借りも無く全部新しく書く
- 問い 1・3・4・6 も写さずに新しく書く。
- ✅ 量の上限の問題が消える・置き場が最も小さい ❌ 「写した分の 1 行あたりの手間」（要件 10.3）が測れない。問い 10 の「対応表だけで移せるか」も確かめられない。

#### 案 C: 独立のワークスペース＋写し（問い 1・3・4・6）＋path で借りる（問い 7 の今の側）（要件の形）
- 写しは置き場の中の小さなクレートに入れ、本線の `wintf`・`areka-emo-text` は path で 0.62 のまま借りる。
- ✅ 要件どおり・手間の実数が取れる・本線を汚さない ❌ 量が 1,500 行を超える（2.3）。❌ 借りたクレートの推移の依存の版は、置き場の `Cargo.lock` で新しく解かれ、本線の `Cargo.lock` と違いうる（`bevy_ecs` などの細かい版）。本線の `Cargo.lock` を写して種にすれば揃えられる。

**見立て**: 案 C が要件の形で、残る決めは「量の数え方」と「`Cargo.lock` の種」の 2 つ。

### 5.2 束縛の作り方（要件 1.6・問い 2・3）

| 案 | 中身 | ✅ | ❌ |
|---|---|---|---|
| 生成して `build.rs` で `OUT_DIR` へ | 各クレートの `build.rs` が `windows-bindgen` を呼ぶ | 見張りの 1,000 行に当たらない（`target` の下）。フィルターを直すとすぐ組み直る | 上流の文書の推す形（「公開するライブラリは別の道具で生成してコミット」）と違う。ビルドのたびに生成が走る |
| 生成してコミット（1,000 行で分ける） | 置き場の中の小さな道具が生成し、`--package`（名前空間ごとに 1 ファイル）か手で分けて置く | 移行の本体の形（束縛を作る道具＋コミット）に近い。差分が読める | 1 ファイルが 1,000 行を超えやすい（D2D・DWrite の名前空間は大きい）。`--package` の分け方で収まるかは未確認 |
| 両方を試す | 問い 1 は `build.rs`、問い 2・3 はコミット、など | 本体の 1 本目（束縛を作る道具）の材料が増える | 手間が増える |

### 5.3 問い 2 の 3 つの置き場（参照の口が無いと分かった後の形）

| 案 | 組み方 | 重なる束縛 | 型の受け渡し |
|---|---|---|---|
| 1 クレートごとに束縛を持つ | `wintf` 役と `emo-text` 役がそれぞれ `ID2D1DeviceContext` などを生成 | 両方に同じ型が出る | 型が別なので、渡す口で `cast`（QueryInterface）か `from_raw` が要る。`windows-text` の `as_interface` と同じ形 |
| 2 `wintf` 役が束縛を公開 | `emo-text` 役は `wintf` 役の型を使う | 無い | 型がそのまま渡る。ただし `emo-text` 役だけが使う API（`IDWriteTypography`・`IDWriteTextRenderer1` など）も `wintf` 役の束縛に入れる必要がある（参照の口が無いので、`emo-text` 役が自分で生成すると型が重なる） |
| 3 共有の束縛のクレート | 束縛だけのクレートを 1 つ置き、両役が使う | 無い | 型がそのまま渡る。フィルターは全クレートの要る API の和になる |

仮定 2 の「参照の口が無いと分かった時点で、それを前提とする案を外す」に当たる案は、3 つのうちどれにも直には無い（どの案も参照の口なしで組める）。口が無いことで変わるのは、案 2 で `wintf` 役の公開の束縛が `emo-text` 役の要る API まで抱えることと、案 1 で型の重なりが避けられないこと。

### 5.4 問い 9 の数え方

| 案 | 中身 | ✅ | ❌ |
|---|---|---|---|
| grep | `windows::` の綴りを数える | 速い・道具が要らない | glob の取り込み 110 か所で、使う名前が字面に出ない。関数・型・定数・メソッドに分けられない |
| rust-analyzer の SCIP の出力 | 本線を `rust-analyzer scip` で索引し、`windows` クレートの記号への参照を数える | 名前の解決がコンパイラと同じ・種類（関数・型・定数・メソッド）が分かる・だれが走らせても同じ数 | 本線の全体を索引する重さ（`tech.md` の `-j 4` の決まりに近い配慮が要る）。出力を読む道具を置き場に書く |
| 失敗を起点にコンパイル | `windows` を外したつもりで組み、出た「見つからない」を集める | 移行の本体の「束縛を作る道具」と同じ道 | 本線の写しが要り、要件 1.3・1.8 と衝突する。何周も回す |
| nightly の `-Zunpretty` などで展開 | 取り込みを解いた形を出す | | 種類の分けが手作業になる |

本線の作業木を読むだけの数え方（grep・SCIP）なら、要件 10.2 の「数えた本線のコミットを書く」と両立する。

---

## 6. 規模と危険

- **規模: L（1〜2 週）**。問いが 10 あり、そのうち 3 つ（問い 3・4・7）が実の GPU、2 つ（問い 4・7）が実の窓と画面の操作、1 つ（問い 10）が別の実装係の走りを要する。上流の取り寄せ（約 1.4 GB）とクロスの 3 ターゲットのビルドも重い。要件の想定の M（10〜14 タスク）は、問い 7 か問い 10 を軽くするか、2 本に分けないと詰まる見込み。
- **危険: 高い**。上流が未公開で、2 日で先頭が動いた（`520097a3` → `f37e2f90`）。参照の口が消えているなど、brief の時点の前提が既にずれた所がある。画面の操作は排他テストの机の順を待つので、待ち時間が読めない。

---

## 7. 設計への申し送り

### 7.1 推す方向（決めではない）

- 置き場は案 C（5.1）。置き場の中に `.cargo/config.toml` を置き、`target-dir` を作業木の `target\pilot-windows-rs-next\` へ向ける。
- 束縛は、問い 1 を `.sys()`、問い 2・3 を既定と `.minimal()` の 2 形で生成する。生成の置き方（`build.rs` かコミットか）は 5.2 の比べを設計で決める。
- 要件 4.3 の答えは 3.2 の根拠（`cli.rs` の旗の一覧・`lib.rs` の `ReferenceStage` の作り手）を一次記録に引けば足りる見込み。
- 問い 5 と対応表の `HWND`・`==` の行は、束縛の形ごとに答えを分けて書く（3.3・3.4）。
- 今の areka の縦書きの画素は、`decoration_readback` と同じ組み方（公開の API＋`present_frame`＋読み戻し）を置き場で新しく書いて取る。
- 見張りの禁止（2.2 の表）を設計の「置き場の決まり」に並べ、タスクのレビューで見る。

### 7.2 Research Needed（設計の段で確かめる）

1. 3.5 の 1〜5（マクロの `_Impl`・i686 の `…LongPtrW`・`.implement` の継ぎ・git の依存の解決・ツールチェーン）。
2. `--package` で生成した名前空間ごとのファイルが 1,000 行に収まるか（D2D・DWrite・WUC）。
3. 生成した `ID2D1CommandList` が形ごとに `PartialEq` を持つか（3.4）。
4. `windows-text` の `TextLayout` を、こちらの D2D の装置で `DrawTextLayout` に渡せるか（工場が別でも描けるか）。
5. cargo の git の取り寄せを浅くする手（`net.git-fetch-with-cli`・nightly の浅い取り寄せ）が使えるか、使わなくてよいか。
6. SCIP の出力から `windows` の記号を種類ごとに数える道具の大きさ。

---

## 8. 要件の討議へ出す議題の候補

1. **写す量の上限（要件 1.8）**: 実数は問い 1・3・4・6 で約 1,800 行（テスト込み）、問い 10 の 2 つ目の写しを足すと約 2,900 行。上限を「テストを除く」「問い 10 の写しは数えない」などに数え直すか、上限を上げるか、写す範囲を削るか（例: `lib.rs` の試験の約 320 行を写さない・問い 6 の `IDWriteTextRenderer1` は写さず新しく書く）。
2. **要件 4.3 の答えがもう出ている件**: `windows-bindgen` 0.100.0 と master に参照の口は無い（3.2）。要件 4.3 を「根拠を一次記録に書く」に絞り、仮定 2 の「外す案」の書き方（どの案も口なしで組める）を直すか。3 案とも組む（仮定 2 のまま）か。
3. **`HWND` の書き方（要件 6.1・12.2）**: 「生ポインタの別名」は `.sys()`／`.minimal()` のときだけ当たる（3.3）。要件の文を「束縛の形ごとに」と直すか。
4. **引く上流のコミット（仮定 3）**: master は 11-14 のうちに `f37e2f90` へ進んだ。「着手の時の先頭」のままにするか、設計の時点で 1 つに決めて要件か設計に書くか。
5. **束縛の置き方（要件 1.6）**: `build.rs` で `target` の下へ作るか、コミット（1,000 行で分ける）か。移行の本体の 1 本目（束縛を作る道具）の材料としては、コミットの形を 1 度は通しておく価値がある。
6. **問い 9 の数え方（要件 10.2）**: glob の取り込み 110 か所のため grep では分けて数えられない。rust-analyzer の SCIP を使う手順を要件に書くか、数え方は設計に任せるか。
7. **置き場の `Cargo.lock` の種（要件 4.1・8.1）**: 借りた `wintf`・`areka-emo-text` の推移の依存の版を本線と揃えるため、本線の `Cargo.lock` を写して始めるか。
8. **上流の取り寄せの置き場（要件 15.2）**: cargo の git の写し（約 1.4 GB）は `CARGO_HOME` に入り、作業木の外になる。「ビルドの出力」とは見ないでよいか。
9. **規模と spec の本数（仮定 1）**: 見立ては L。1 本のまま 10〜14 タスクに収めるなら、問い 7 を別の先進坑へ出す案（仮定 1 の案）か、問い 10 を問い 1 の写しの一部（例えば `util/window.rs` だけ）に絞る案がある。
10. **問い 6 の範囲（要件 7.1・7.2）**: SHIORI の `#[interface]` はマクロで、bindgen の出力ではない。PR 5299 の `HRESULT` の変更がマクロにも及ぶかは未確認（3.5 の 1）。及ばないなら問い 6 の SHIORI の半分は「今のまま通る」の確かめで済む。
11. **i686 の `…LongPtrW`（要件 3.2・9.1）**: 問い 1 の写しの i686 の主な危険。止まったときの直し方（写しの側で `cfg` で分ける）を「直す」の見立てで許すか。
12. **見張りの禁止を要件に書くか（要件 1.7）**: `env-filter` を宣言しない・捕捉先を差す語を綴らない・`std::env::temp_dir` を使わない・検体のパスを組まない、の 4 つは要件 1.7 の「見張りを赤にしない」に含まれる。名指しで要件に足すか、設計に並べるだけにするか。
