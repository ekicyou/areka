# ギャップ分析（＝見直しの audit）: audit-wintf-runtime

- 日付: 2026-11-05
- 測った基点: main `88eb0a43`（この枝の先頭 `2859a137` は spec の文書を足しただけで、`crates/wintf/` は同じ）
- 物差し: ponytail 5.3.2 の `/ponytail:ponytail-audit`。brief のとおり 2 つを外した（「最大 20 件」を外して全件を出す・「読まなかった所」を残さず全部読む）
- やり方: 範囲のファイルを 7 つの束に分け、束ごとに読み手を立てて全行を読ませた。返ってきた指摘は、主なものを根拠の行まで読み直して確かめた（下の「確かめた指摘」）。cargo のビルドとテストは回していない（読みと grep だけ）

---

## 1. 範囲の確定（要件 1）

- 範囲: `git ls-files crates/wintf` のうち `crates/wintf/src/ecs/` の下を除いたもの全部。**160 ファイル**（要件の文書の数と一致）。`vendors/` は入らない。
- 読んだ数: **160 / 160**（範囲の数と一致）。

| 束 | 中身 | ファイル数 | 行数 |
|---|---|---:|---:|
| 1 | `src/runtime/`（11） | 11 | 4,272 |
| 2 | `src/com/`（11） | 11 | 2,643 |
| 3 | `Cargo.toml`・`README.md`・`build.rs`・`examples.manifest`・`src/api*.rs`（3）・`src/lib.rs`・`src/numerics/`（6）・`src/test_crash_report.rs`・`src/win_state.rs`・`src/win_style*.rs`（2） | 18 | 4,175 |
| 4a | `tests/assets/`（13・画像などは種類と大きさを確かめ、参照を grep）・`tests/*.rs` の入口（8）・`tests/com/`・`tests/drag/`・`tests/ecs/` | 38 | 約 4,600 |
| 4b | `tests/graphics/`・`tests/visual/`・`tests/widget/`・`tests/window/` | 35 | 6,155 |
| 5 | `tests/layout.rs` と `tests/layout/` | 29 | 5,992 |
| 6 | `examples/`（`dcomp_demo.jpg` は種類と大きさを確かめ、参照を grep） | 18 | 5,211（.rs 17 本）＋ jpg 1 枚 |
| 計 | | **160** | |

- どの `tests/**/*.rs` も入口の `#[path] mod` から届いている（取り残されたファイル 0・存在しないファイルを名指す宣言 0）。

## 2. 想定した負荷と、このクレートがしていること

- **何をするクレートか**: wintf は bevy_ecs の上に Win32・Direct2D・DirectWrite・WIC・Windows.UI.Composition（WUC）を載せた Windows 用の UI の枠組み。crates.io に `wintf 0.1.0` として出ている。この spec の範囲は、ECS の外にある刻みの土台（`runtime/`）・COM の薄い包み（`com/`）・窓の型（`win_style`・`win_state`）・Win32 の小さな口（`api.rs`）・数の型（`numerics/`）と、テストと example。
- **想定した負荷**: 利用者 1 人のデスクトップ上の 1 プロセス。窓は数枚。UI スレッドは画面の更新周期（60〜144Hz）で刻み、vblank のスレッドが 1 本。何時間も何日も動き続ける。読む絵はゴーストやシェルのフォルダの第三者の資産。

## 3. 指摘の一覧（要件 2）

### 件数

| 組 | 件数 |
|---|---:|
| 必ず直す（Must fix） | **5** |
| 直したほうがよい（Should fix） | **63** |
| できれば（Nice to have） | **51** |
| 計 | **119** |

- 番号は 3 つの組を通した通し番号（audit の作法）。
- 印の意味:
  - 〔公開〕＝直すと crates.io の `wintf` の公開の口（クレートの根から届く型・関数・定数・マクロ）が消えるか形が変わる。要件 4.4 の一覧の候補。
  - 〔DWMFLUSH〕＝`wintf-dwmflush-wait-compensation` の直す中身と重なる（要件 3.6）。
  - 〔描画の木 傷 n〕＝`wintf-visual-tree` の brief の「直すべき傷」n と重なる（要件 3.5）。
  - 〔描画の木 申し送り〕＝`numerics` のうち描画の木だけが使う働きで、未使用を理由に消さない（要件 3.4）。
  - 〔残す候補 ⑴/⑵/⑶〕＝要件 3.2 の「直さずに残してよい 3 種類」に当たりうる。要件の討議で開発者の裁定を仰ぐ。
  - 〔範囲の外〕＝直しの本体が別の見直しの spec の受け持つファイルにある（要件 6）。

---

### 必ず直す（Must fix）

1. **COM の参照を数えずに写す公開の型が、安全なコードで解放後の参照を起こす**（`crates/wintf/src/com/d2d/command_types.rs:L8-L14`・同じファイル L16-L584 の `ManuallyDrop<COM>` を持つ全部の型）〔公開〕
   - 何か: `dup_com` は COM のポインタを `transmute_copy` で写すだけで参照の数を増やさない。公開の安全な `X::new(&iface)` がその写しを公開の欄に入れる。
   - 困ること: `let cmd = { let b = 筆; FillRectangle::new(r, &b) };` のあとで `cmd.brush` を使うと、解放済みの物を触る（unsafe を書いていない側で未定義の動作）。`#[derive(Clone)]` の `clone()` は参照を増やすが減らさないので漏れる。注記は「この関数は unsafe」と言うが、関数は unsafe でない。
   - 直し方: 13 番（`RecCommandSink` ごと消す）で消える。残すなら欄を素の `T` にして `dup_com` を消す。
   - 直さないと: crates.io の利用者が、自分では unsafe を書かずに落ちる。

2. **vblank のスレッドが、`DwmFlush` の失敗の間は一度も起こさず、待たずに戻る間は空回りする**（`crates/wintf/src/runtime/tick_bridge.rs:L125-L143`・注記 L152-L156・`runtime/mod.rs:L337`・`L365`）〔DWMFLUSH〕
   - 何か: `vsync_loop` は `Ok` なら起こしてすぐ次を呼び、`Err` なら 15ms 眠るだけ。
   - 困ること: `Err` が続くあいだは `notify` を撃たないので、ECS の刻み・窓の書き込み・アニメが全部止まり、ログも出ない。`Ok` でも待たずに戻る場面（消灯・ロック・リモートデスクトップ）では CPU が張り付く。`AsyncTickTask` の注記と `mod.rs` の 2 か所は「60Hz の刻み」「固定 60Hz のフォールバック」と書くが、実際と違う。
   - 直し方: `wintf-dwmflush-wait-compensation` の brief の形（呼ぶ前後の時間で判定・不足分だけ眠る・ループの最後で必ず 1 回起こす・注記を直す）。`mod.rs` の 2 か所の注記もいっしょに直す。
   - 直さないと: ロック画面やリモートデスクトップのあいだ、アプリが止まるか CPU を食い続ける。

3. **位置だけを指定した窓が、生成のときに位置を無視される**（`crates/wintf/src/runtime/window_factory.rs:L252-L270`）
   - 何か: 生成直後の `SetWindowPos` は「位置も大きさも `CW_USEDEFAULT` でないとき」だけ呼ぶ。
   - 困ること: `WindowPos::default()` は位置も大きさも `CW_USEDEFAULT`（`ecs/window/window_pos/mod.rs:L76-L87`）。`WindowPos { position: Some((500,50)), ..Default::default() }`（`examples/clip_demo.rs:L266-L269`・`wndproc_bridge_drag_test_support.rs:L230-L236` がこの形）は OS の既定の位置に `SW_SHOW` で出て、次の刻みで (500,50) へ飛ぶ。`to_window_coords_for_creation`（`window_pos/mod.rs:L378`）も片方だけのときは変換しない。注記（L14-L15）の「出た瞬間の飛びを避ける」と食い違う。
   - 直し方: どちらか片方でも指定があれば呼び、位置が既定なら `SWP_NOMOVE`、大きさが既定なら `SWP_NOSIZE` を足す。位置だけのテストを 1 本足す。
   - 直さないと: 位置だけで置いた窓が一瞬違う所に出てから跳ぶ。

4. **文字の塊の数を問い合わせる口が DirectWrite の失敗を捨て、空の結果を成功として返す**（`crates/wintf/src/com/dwrite.rs:L153-L179`・`let _ =` は L158 と L176）
   - 何か: `get_cluster_metrics`／`get_cluster_count` は、まず大きさだけを問い合わせ、その戻り値を捨てる。
   - 困ること: 正常なら「入れ物が足りない」の失敗が返るが、ほかの失敗（レイアウトや書体の失敗・メモリ不足）でも数が 0 のまま `Ok(空)`／`Ok(0)` になる。本番の呼び手（`ecs/widget/text/typewriter_layout.rs:193`・`typewriter_draw.rs:114`・`areka-emo-text/src/draw_metrics.rs:304`）は何も並べず何も描かず、ログも出ない（「記録の無い失敗の道」）。
   - 直し方: 問い合わせの戻り値を見て、`Ok` なら空、「入れ物が足りない」なら続ける、それ以外は `error!` を残して `Err` を返す。失敗の枝のテストを足す。
   - 直さないと: バルーンの行が理由なく空になる。

5. **テストが公開の口のバグ（百分率が 100 倍）を正しいものとして固定し、その追跡の印も実在しない**（`crates/wintf/tests/layout/dimension_conversion_test.rs:L6-L7`・`L84-L97`・`L120-L130`）〔範囲の外〕〔公開〕〔残す候補 ⑴〕
   - 何か: `LengthPercentageAuto::Percent(50.0)` と `LengthPercentage::Percent` が taffy の `percent(50.0)`＝5000% になることを固定するテスト。
   - 困ること: 型の文書は「0.0〜100.0 で指定」と書き（`src/ecs/layout/dimension.rs:L113`・`L120`）、`Dimension::Percent` は 100 で割る（同 L104）。margin・padding・inset を百分率で書いた利用者は 100 倍の値を受け取る。テストが引く「proposals.md P49」はリポジトリに無い（`git ls-files | grep proposals.md`＝0 件）。
   - 直し方: 本番の 2 つの `From`（`dimension.rs:L155-L163` ほか）に `/ 100.0` を足し、テストの期待を裏返す。本番のファイルは `audit-wintf-layout-widget` の受け持ち。
   - 直さないと: 百分率の余白を使った最初の利用者が壊れた見た目を受け取り、テストがそれを守る。

---

### 直したほうがよい（Should fix）

#### 本番のコード（`src/`・`Cargo.toml`）

6. **窓の生成に失敗した実体を、毎フレーム作り直そうとして毎回 `error!` を出す**（`crates/wintf/src/runtime/window_factory.rs:L147-L153`）
   - 何か: `LibWindow::new_ex` が失敗すると、ログを出して戻るだけで実体に印を残さない。
   - 困ること: `create_windows`（`ecs/window/window_system.rs:L35`・毎刻みの `UISetup`）は `With<Window>, Without<WindowHandle>` で同じ実体を拾い直す。USER のハンドルの上限などで失敗し始めると、毎秒 60〜144 回 `CreateWindowExW` と `error!` が繰り返される。
   - 直し方: 失敗したら 1 度だけログを出し、`Window` を外すか「生成に失敗」の印を付けて問い合わせから外す。
   - 直さないと: 1 枚の失敗でログがあふれ、失敗する Win32 の呼び出しが画面の周期で続く。

7. **UI スレッドが別の COM の部屋で初期化済みでも、黙って受け入れる**（`crates/wintf/src/runtime/mod.rs:L157-L162`）
   - 何か: `CoInitializeEx(MTA)` が `RPC_E_CHANGED_MODE` を返しても成功とみなし、ログを出さない。
   - 困ること: 利用者が先に STA で初期化していると、WUC と D2D の物が全部 STA の上で作られ、後の不具合の手がかりが何も残らない。
   - 直し方: その場合に `warn!` を 1 行出す（戻り値は今のまま）。
   - 直さないと: 部屋の食い違いが原因の不具合を診断できない。

8. **vblank のスレッドの panic が trace でしか記録されない**（`crates/wintf/src/runtime/tick_bridge.rs:L99-L101`）〔DWMFLUSH と同じファイル〕
   - 何か: `join()` が `Err`（スレッドが panic）でも `trace!` だけ。
   - 困ること: このスレッドが落ちると UI は二度と刻まれないのに、既定のログには何も出ない。
   - 直し方: `error!` にする。
   - 直さないと: アプリが止まった理由がログに残らない。

9. **窓の生成の後の Win32 の失敗を、ログなしで捨てる**（`crates/wintf/src/runtime/window_factory.rs:L172-L174`（`SetParent`）・`L240-L250` と `L259-L269`（`SetWindowPos`）・`L275-L277`（`SetWindowTextW`））
   - 何か: どれも `let _ = ...`。`SetParent` の注記は「無視」と明言する。
   - 困ること: `Window { parent: Some(古い HWND) }` で `SetParent` が失敗すると、窓は黙って最上位のまま残る。
   - 直し方: 各呼び出しを `if let Err(e) = ... { error!(...) }` にし、今どおり先へ進む。
   - 直さないと: 親子や配置の食い違いを追えない。

10. **窓の登録簿をその場で作る逃げ道が debug でしかログを出さない**（`crates/wintf/src/runtime/window_factory.rs:L189-L200`）
    - 何か: `WindowRegistry` が無いと、終了の仕掛けの無い登録簿を作って `debug!` だけ出す。注記（L193-L195）は「結線の不備を検知できるよう明示ログを残す」と言う。
    - 困ること: もし通ると、`ExitPolicy::OnLastWindowClose` で最後の窓を閉じてもアプリが終わらず、既定のログには何も出ない（今の本番とテストでは通らない）。
    - 直し方: `error!` にする（または枝を消して `error!` を出し、窓を捨てる）。
    - 直さないと: 結線の後退が「終わらない」としてだけ現れる。

11. **クリック透過の中継の「run の後は止まる」の判定にテストが無い**（`crates/wintf/src/runtime/mod.rs:L390-L408`）
    - 何か: `run_async_tick` の近い写しで、自分の `running` の判定（L402-L405）を持つ。刻みの側の写しにはテストがある（`tick_bridge.rs:L571-L611`）が、こちらには無い。
    - 困ること: L402 の判定を消しても赤にならない。消すと run の後に登録簿の無い World を読みに行く（2026-09-25 の事故と同じ型）。
    - 直し方: 中継を `run_async_tick` の隣の小さな関数へ出し、刻みの側のテストを写す。
    - 直さないと: 守りの無い後退の道が残る。

12. **公開の `drain_dispatcher_queue` は呼び手が 0 で、しかも WM_QUIT を捨て、状態の取得の失敗を成功として返す**（`crates/wintf/src/com/wuc.rs:L35-L118`・テスト `L237-L292`・使う import `L13-L15`）〔公開〕
    - 何か: DispatcherQueue を有限の時間で空にする終了の道具。
    - 困ること: 呼び手は 0（`git grep` で範囲の外も 0 件。`examples/wuc_spike.rs` は自前の写しを持つ）。中身に 2 つの穴: `Status()` の失敗を完了とみなしログなしで `true` を返す（L88）・`pump_current_thread_messages` が `WM_QUIT` を取り除いて戻り、投げ直さない（L111-L113）。
    - 直し方: 消す（関数・純関数 `drain_step`・テスト・import）。残すなら `Status()` の失敗で `error!`＋`false`、`WM_QUIT` は `PostQuitMessage` で投げ直す。
    - 直さないと: 2 つの穴を抱えた死んだ公開の口が、終了の道で使われていると誤解されたまま残る。

13. **`RecCommandSink` は描画命令を読めない入れ物へ書くだけで、読み出しの口は `todo!()`**（`crates/wintf/src/com/d2d/command_sink.rs:L1-L470`・`command_types.rs:L1-L584`・`d2d/mod.rs:L1-L4`）〔公開〕
    - 何か: `ID2D1CommandSink5` を実装し、再生された命令を全部 `RefCell<Vec<DrawCommand>>` へ写す。
    - 困ること: 公開の `commands()` は `todo!()`＝呼ぶと panic。ほかに入れ物を見せる口は無い。本番の利用者は 0（`RecCommandSink`・`command_types::`・`dup_com` は `tests/com/` だけ。`ecs/visual/draw/` の `DrawCommand` は別の型）。約 1,050 行の unsafe な公開のコードが、テストされているように見えて何も確かめられていない。
    - 直し方: 2 ファイルと `d2d/mod.rs` の 4 行を消す。テストは 24 番で消す。
    - 直さないと: 1 番の未定義の動作ごと、使えない公開の口が出荷され続ける。

14. **COM のアニメの包みは本番で一度も使われない**（`crates/wintf/src/com/animation.rs:L1-L182`・`com/mod.rs:L1`）〔公開〕〔範囲の外（根の `Cargo.toml` の機能）〕
    - 何か: `IUIAnimation*` の拡張の trait。
    - 困ること: テスト（`tests/com/animation_test.rs`）の外の利用者は 0。根の `Cargo.toml:L83` の `Win32_UI_Animation` 機能はこのファイルのためだけにある。
    - 直し方: ファイル・`pub mod animation;`・テストを消し、根の `Cargo.toml` から機能を外す。
    - 直さないと: 誰も使わない公開の面と、コンパイルの時間が残る。

15. **`load_bitmap_source` が求めた α の有無を、ただ 1 つの本番の呼び手が捨て、同じ判定が areka-emo-atlas に写されている**（`crates/wintf/src/com/wic.rs:L11-L116`・`com/wic_tests.rs:L1-L27`）〔公開〕
    - 何か: 変換の前の画素の形式とパレットから α の有無を決めて返す。
    - 困ること: 本番の呼び手 `ecs/widget/bitmap_source/bitmap_source.rs:L111` は `_has_alpha` で捨てる。それでも番号つきの画像のたびにパレットを作って写す。areka の本当の読み込み（`areka-emo-atlas/src/decode/wic_arm.rs:L77-L83`）は同じ形式の一覧と同じ規則を自前で持つので、直すときは 2 か所になる。（読み手の補足: WIC が CMYK＋α などの形式で返す画像では、どちらの一覧も α 無しと答える見込み。この機械では測っていない。）
    - 直し方: `load_bitmap_source` の戻りを `IWICBitmapSource` だけにし、L74-L116 と `wic_tests.rs` を消す（テスト `tests/com/wic_test.rs` の 6 本と `tests/widget/bitmap_source_integration_test.rs` も合わせる）。組を残すなら、手書きの一覧を `IWICPixelFormatInfo2::SupportsTransparency` に置き換える。
    - 直さないと: 毎回の無駄な仕事と、ずれていく 2 つの一覧が残る。

16. **`D2D1CommandListExt::open` は必ず失敗する見せかけの口**（`crates/wintf/src/com/d2d/mod.rs:L73-L92`）〔公開〕
    - 何か: いつも `E_NOTIMPL` を返す公開のメソッド。
    - 困ること: `cl.open()?` は必ず失敗する。使うのは「失敗すること」を確かめるテスト（`tests/com/d2d_ext_test.rs:L95`）だけ。
    - 直し方: trait と実装から `open` を消し、そのテストを消す。`close` は areka-emo-present が使うので残す。
    - 直さないと: 罠の口が公開のまま残る。

17. **`WinState`／`SimpleWinState` は使われない古い型で、完了した spec がすでに消すと書いている**（`crates/wintf/src/win_state.rs:L1-L191`・`src/lib.rs:L8`・`L13`）〔公開〕
    - 何か: ECS より前の「HWND・DPI・マウスの追跡」の trait と、その実装 1 つ。
    - 困ること: 範囲の外も含めて利用者 0（`git grep` 0 件）。完了した `dpi-coordinate-transform-survey` の report.md:108-118（G8）が「ECS へ移ったら消す」と書き、その条件は満たされた。ファイル全体の `#![allow(unused_variables)]` が警告を隠す。
    - 直し方: ファイルと `lib.rs` の 2 行を消す（7 本のテストも消える）。
    - 直さないと: ECS の `DPI` の隣に 2 つ目の状態の型が残る。

18. **`WinStyle` の組み立て役は使われず、位置・大きさ・親の欄は書くだけで何にも効かない**（`crates/wintf/src/win_style.rs:L7-L350`・`L426-L446`・テスト `L452-L667`）〔公開〕
    - 何か: 窓の型の組み立て役と `commit(hwnd)`。
    - 困ること: 利用者 0。`WinStyle::WS_POPUP().position(10,10).size(100,100).commit(h)` は位置も大きさも変えない（欄を読む所が無い）。`commit` は `SWP_FRAMECHANGED` を呼ばないので枠が更新されない。注記の「SetWindowLongW」も実際（`SetWindowLongPtrW`）と違う。
    - 直し方: `WinStyle`・`set_style`・`set_ex`・そのテストを消す。本番が使う `apply_click_through`・`apply_layered_companion`（L352-L424）とそのテストは残す。`#![allow(non_snake_case)]` も外す。
    - 直さないと: 効かない形を約束する公開の型が残る。

19. **使われない依存と置き場所の違う依存**（`crates/wintf/Cargo.toml:L27`（`nonmax`）・`L38`（`rand`）・`L21`（`bevy_app`））〔範囲の外（根の `Cargo.toml:L23`・`tech.md`）〕
    - 何か: `nonmax` はリポジトリ全体で使う所 0。開発用の `rand` は wintf の中で 0（dola は自前で持つ）。`bevy_app` は `src/` で 0、テスト `tests/graphics/surface_optimization_test.rs` だけ。
    - 困ること: 依存の木と監査の面が要らずに広い。
    - 直し方: `nonmax` と `rand` の行を消す（根の `[workspace.dependencies]` の `nonmax` も）。`bevy_app` は開発用の依存へ移す。`tech.md` の該当の行を直す。
    - 直さないと: 死んだ依存の固定が残る。

20. **`ambassador` は誰も委譲しない trait に印を付けるためだけに入っている**（`crates/wintf/Cargo.toml:L18`・`src/win_state.rs:L4`・`L8`・`src/com/d2d/mod.rs:L6`・`L94`）〔公開（隠れたマクロ）〕
    - 何か: `#[delegatable_trait]` は `WinState` と `D2D1DeviceContextExt` の 2 か所だけ。
    - 困ること: `#[delegate(` はリポジトリに 0 件。効き目は `ambassador_impl_*` のマクロを外へ出すことだけ。
    - 直し方: 17 番の後、`d2d/mod.rs` の印と `use ambassador::*` を消し、依存の行を消す（`tech.md` の行も）。
    - 直さないと: proc-macro の依存が隠れたマクロのためだけに残る。

#### 描画の木と重なる数の型（`src/numerics/`・申し送り）

21. **「軸に沿うか」の判定が 3 通りあり、答えが食い違う**（`crates/wintf/src/numerics/matrix3x2.rs:L10-L12`・`numerics/aabb.rs:L133`・範囲の外 `ecs/visual/draw/builder.rs:268`）〔描画の木 傷 3〕〔描画の木 申し送り〕
    - 何か: `axis_aligned` と `Aabb::transform` の近道は「ちょうど 0」、描画の木の `is_rect_preserving` は 1e-4 の許容で 90 度の回転も認める。
    - 困ること: 1e-6 ラジアンの回転は一方で「沿わない」、他方で「沿う」。90 度の回転も片方だけが認める。
    - 直し方: `wintf-visual-tree` の傷 3 として、あちらで 1 つにまとめる。この spec では消さない。
    - 直さないと: 描画の木を本番へ上げたとき、切り抜きの形の選び方が場所で変わる。

22. **矩形の「空」「交わり」「変形」が `Aabb` と描画の木で二重にあり、端の値で食い違う**（`crates/wintf/src/numerics/aabb.rs:L81-L83`・`L91-L98`・`L127-L160`・範囲の外 `ecs/visual/draw/builder.rs`）〔描画の木 傷 4〕〔描画の木 申し送り〕
    - 困ること: `Aabb::new(0,0,NaN,10)` は numerics では空、描画の木では空でない。空の矩形を単位行列で変形すると、numerics は空、描画の木は平面全体を返す。
    - 直し方: `wintf-visual-tree` の傷 4 として、numerics の側へ寄せる。
    - 直さないと: 間引きが端の値で逆の答えを出す。

23. **`numerics` に 1 本もテストが無い**（`crates/wintf/src/numerics/` 全部・特に `transform2d.rs:L63-L95`・`aabb.rs:L127-L160`）〔描画の木 申し送り〕
    - 何か: `Transform2D::pivot_srt`（回転の符号・基準点の平行移動）・`to_affine` の掛ける順・`Aabb::transform` の外接矩形。
    - 困ること: `transform2d.rs:L70` の `m21` の符号を裏返しても何も赤にならない。
    - 直し方: `wintf-visual-tree` の「純粋なテスト」で網羅する（あちらの Desired Outcome に既にある）。
    - 直さないと: 描画の木を本番へ上げるときの土台の計算が無防備のまま。

#### テスト（`tests/com`・`tests/drag`・`tests/ecs`・`tests/win_app.rs`）

24. **`RecCommandSink` のテストは記録の中身を確かめられず、13 番の死んだコードを生かしている**（`crates/wintf/tests/com/command_sink_test.rs:L1-L157`・`command_types_test.rs:L1-L167`・`d2d_ext_test.rs:L216-L219`・`L325-L367`）〔公開（13 番と組）〕
    - 困ること: `SetTransform` が別の命令を積むように壊しても全部緑。`#[should_panic]`（期待の文言なし）が `todo!()` を固定している。`dup_com` の参照の数のテストは、正しく数える作りでも通る。
    - 直し方: 13 番と合わせて 2 ファイルと `com.rs` の宣言（L4-L7）を消し、`d2d_ext_test.rs` の L216-L219・L325-L367 を消す（L121-L215 は「拡張の包みで記録して閉じる」テストとして残す）。
    - 直さないと: 約 1,050 行の使われない unsafe な公開のコードが「テスト済み」に見える。

25. **葉の変化が根まで伝わるかのテスト 2 本が、伝える係が何もしなくても通る**（`crates/wintf/tests/ecs/tree_propagation_test.rs:L176-L228`・`L235-L281`）
    - 何か: `mark_dirty_trees` を回したあと、新しく組んだ確かめの system で根の `ArrangementTreeChanged` の `is_changed()` を見る。
    - 困ること: 新しく組んだ system の最初の 1 回は、それまでに入った部品を全部「変わった」と見る（bevy_ecs 0.19.1 `function_system.rs:737`）。根の部品は生成のときに入っているので、`mark_dirty_trees` の中身を空にしても緑。
    - 直し方: 回す前に `world.read_change_tick()` を控え、根の部品の変更の刻みが控えより新しいことを確かめる。確かめの system と `DirtyProbe` は消す。
    - 直さないと: 葉が動いても子のレイアウトが古いまま、の後退がテストを抜ける。

26. **CaptureGuard の「panic でも放す」のテストは CaptureGuard を見ていない**（`crates/wintf/tests/drag/capture_guard_panic_safety_test.rs:L1-L101`・`tests/drag.rs:L2-L3`）
    - 困ること: 1 本目と 3 本目はテストが自前で作った `DropDetector` の drop を見るだけ（Rust が保証すること）。2 本目は panic が捕まったことだけ。`CaptureGuard` の `Drop` から `ReleaseCapture()` を消しても 3 本とも緑。旗の論理は `src/ecs/drag/capture_guard.rs:L80-L110` が既に確かめている。
    - 直し方: ファイルと宣言を消す。
    - 直さないと: 誰も確かめていない保証を 3 本のテストが示しているように見える。

27. **ECS の 3 つのテストファイルは wintf を 1 行も呼ばず、bevy と自前の模型だけを試す**（`crates/wintf/tests/ecs/component_state_pattern_test.rs:L1-L390`・`lazy_reinit_pattern_test.rs:L1-L408`・`resource_removal_detection_test.rs:L1-L339`・`tests/ecs.rs:L2-L7`）
    - 何か: 「作り直しの型をどれにするか」を決めたときの実験が、テストとして残ったもの（`wintf` の語 0 件・大きなコメントの結論の塊）。
    - 困ること: 本番がどう変わっても赤にならない。いくつか（`*_changed_detection`・`test_resource_removal_detection_summary` ほか）は `eprintln!` だけで何も確かめない。
    - 直し方: 3 ファイルと 6 行の宣言を消す。
    - 直さないと: 失敗しえないテストが実行の時間を食う。

28. **画素の写しのテストが、何も写さない包みでも通る**（`crates/wintf/tests/com/wic_test.rs:L302-L326`・`L328-L357`）
    - 困ること: 入れ物は全部 0 で始まり、テストは `Ok` だけを見る。`copy_pixels` を何もせず `Ok` を返すように壊しても緑。
    - 直し方: 全体の写しの後に「0 でない画素がある」を確かめ、部分の写しは全体の写しの該当の画素と比べる（`test_8x8_rgba.png` は 2 画素目から α が 0 でないと確かめた）。
    - 直さないと: 部品と atlas の読み込みが頼る写しの道が無防備。

29. **パレットのテストが別のクレートの中のファイルを読む**（`crates/wintf/tests/com/wic_test.rs:L391-L430`）
    - 困ること: 道は `CARGO_MANIFEST_DIR/../areka-emo-atlas/src/testdata/palette`。wintf は `publish = true` なので、crates.io から取った wintf や、ワークスペースの外へ出した wintf では 6 本とも「ファイルが無い」で落ちる。
    - 直し方: 6 つの見本（計 562 バイト）を `tests/assets/palette/` へ写し、`test_asset_path` で読む（15 番で `has_alpha` を消すならこのテストごと消える）。
    - 直さないと: wintf のテストが areka のワークスペースの中でしか通らない。

30. **「UISetup は単一スレッド」のテストはスケジュールが在ることしか見ず、注記も古い**（`crates/wintf/tests/ecs/world_lifecycle_test.rs:L90-L99`）
    - 困ること: 確かめるのは `get(UISetup).is_some()` だけで、L61-L88 と重なる。注記は UISetup だけと言うが、実際は 6 本（`src/ecs/world/mod.rs:134-177`）。どれかの `set_executor(SingleThreadedExecutor)` を消しても緑。bevy 0.19.1 には実行器を読む公開の口が無い。
    - 直し方: このテストを消す。単一スレッドのスケジュールを固定するには、本番に「単一スレッドの名札の一覧」の定数を置く小さな変更が要る（範囲の外 `ecs/world/mod.rs`＝`audit-wintf-graphics-world` の受け持ち）。
    - 直さないと: 名前が約束する保証を誰も確かめていない。

31. **`tick_dola_animators` のテストが `tick_dola_animators` を呼ばない**（`crates/wintf/tests/ecs/dola_animator_test.rs:L93-L136`・`L158-L162`）
    - 困ること: system の中のループをテストの中に写して回すだけ。L158-L162 は関数のポインタへの型変換だけ。本番の system のループを抜いても緑。
    - 直し方: `Schedule` に本物の `tick_dola_animators` を足して回し、L154-L162 を消す。
    - 直さないと: すべてのアニメを動かす system に結合のテストが無い。

32. **ドラッグ中の大きさのテストが、大きさを更新してもしなくても通る**（`crates/wintf/tests/drag/window_dragging_filter_test.rs:L28-L81`・`L174-L189`）
    - 困ること: 期待の大きさが最初の大きさと同じ 800x600。ドラッグ中の実体を丸ごと飛ばすように壊しても緑（L221-L290 だけが捕まえる）。
    - 直し方: L28-L81 を消す（L221-L290 が両方を違う値で確かめる）。L145-L169 は最初の大きさを別の値にする。
    - 直さないと: 重なったテストが、無い保証を示す。

33. **`WinApp` のテストは COM と DPI の初期化をうたうが、どちらも確かめない**（`crates/wintf/tests/win_app.rs:L13-L20`）
    - 困ること: 確かめるのは `try_borrow_mut().is_ok()` だけ。`with_exit_policy` から `CoInitializeEx` と DPI の設定を消しても緑。
    - 直し方: `CoGetApartmentType` で MTA、`GetThreadDpiAwarenessContext` で per-monitor v2 を確かめる（どちらも試験のスレッドで決まった答え）。
    - 直さないと: 入口の COM の初期化を失っても実機でしか分からない。

#### テスト（`tests/graphics`・`tests/visual`・`tests/widget`・`tests/window`）

34. **GraphicsCore の初期化の冪等のテストが、試す system を一度も走らせていない**（`crates/wintf/tests/graphics/core_ecs_test.rs:L44-L75`）
    - 困ること: `FrameCount` を入れていないので、`init_graphics_core` の引数の検査で失敗し（bevy 0.19.1 は panic せず `Err` を返す）、`let _ = system.run(...)` がそれを捨てる。毎回作り直すように壊しても緑。
    - 直し方: `FrameCount::default()` を入れ、`run` を `expect` し、前後の `d2d_device()` が同じであることを確かめる。
    - 直さないと: 毎フレーム装置を作り直す後退が見えない。

35. **「新しい親へ移る」のテストは、移る前の同期をしておらず、初めての同期しか確かめない**（`crates/wintf/tests/visual/hierarchy_sync_test.rs:L57-L117`）〔範囲の外（本番 `ecs/graphics/systems/visual_sync.rs:L46-L53`）〕
    - 何か: 子を親 1 の下に作り、一度も同期せずに親 2 へ付け替えてから同期し、`parent_visual().is_some()` だけを見る。
    - 困ること: 名前は付け替えを試すと言うが、付け替えの道は通らない。本番は付け替えを検知しない（`visual_sync.rs` の NOTE W3b-V・`hierarchy_reparent_gap_test.rs` がこの穴を固定）。その穴「P47」を持つ生きた spec は無い（完了した `areka-P0-draw-load-parity/design.md:835` だけ）。
    - 直し方: このテストを消す。付け替えの穴の直しは `audit-wintf-graphics-world` の範囲。
    - 直さないと: 付け替えが動くとテストが言い張る。

36. **`owner_window_exists` のテストは本物でなく写しを試す**（`crates/wintf/tests/window/owner_window_exists_test.rs:L7-L32`・`L34-L77`）
    - 困ること: 本番の `wintf::ecs::owner_window_exists`（`src/ecs/graphics/visual.rs:38`）を一度も呼ばない。本番を壊しても 5 本とも緑。
    - 直し方: 写しを消し、`owner_window_exists(&DeferredWorld::from(&mut world), e)` で本物を呼ぶ（`From<&mut World>` は bevy 0.19.1 にある）。
    - 直さないと: GPU の部品を付けるかを決める判定に本当のテストが無い。

37. **切り抜きの同期のテストは何も確かめず、前提も古い**（`crates/wintf/tests/graphics/clip_sync_system_test.rs:L1-L7`・`L64-L183`）
    - 困ること: 4 本とも何も確かめない。本番は COM の失敗を全部 `error!` して続けるので、`build_clip` がいつも失敗しても緑。注記は「InsetClip は書くだけ」「SetLeft/SetRight」と言うが、本番は角丸の図形の切り抜き。最後の 1 本は部品が空なので別の理由で通る。
    - 直し方: 各 `schedule.run` を `log_capture_kit::count_levels` で包み、`error` が 0 を確かめる（GPU の持ち主のスレッドの中で）。最後の 1 本は消す。
    - 直さないと: WUC の切り抜きの後退が赤にならない。

38. **`render_surface` のテストは、`render_surface` が変えられない物を確かめる**（`crates/wintf/tests/graphics/surface_systems_test.rs:L319-L348`）
    - 困ること: `SurfaceGraphics` は読むだけで、失敗は `error!`＋`continue`。`begin_draw` がいつも失敗しても緑。注記（L341-L342）の「panic する」も誤り。
    - 直し方: `count_levels` で包み `error` が 0 を確かめる。
    - 直さないと: WUC の描く道の唯一のテストが何も確かめない。

39. **窓の位置の書き込みのテストは何も確かめず、共有の GPU のスレッドに指令を残す**（`crates/wintf/tests/graphics/window_pos_systems_test.rs:L6-L7`・`L192-L269`）
    - 困ること: 注記は「キューを覗く口が無い」と言うが、`wintf::ecs::window::drain_window_pos_commands()`（`src/ecs/window/command.rs:830`・`#[doc(hidden)]` の試験の口）がある。L246 で積んだ指令は、プロセスの寿命の GPU の持ち主のスレッドのキューに残り、後のテストへ持ち越す（注記 L224-L225「スレッドの終わりに捨てる」は誤り）。
    - 直し方: 各回の後に `drain_window_pos_commands()` を取り、空か期待の 1 件かを確かめる。Compositor を作らないので持ち主のスレッドで包むのもやめる。
    - 直さないと: 窓の位置の後退が見えず、残りが後のテストを驚かせる。

40. **見た目の値の同期のテストは何も確かめず、「書くだけ」の前提が古い**（`crates/wintf/tests/visual/property_sync_test.rs:L1-L118`）
    - 困ること: 注記は DComp の時代の「読み戻せない」を前提にするが、今の WUC の `Visual` は `Opacity()`・`Offset()` が読める。不透明度を定数 1.0 にしても 5 本とも緑。
    - 直し方: 同期の後に `Opacity()`・`Offset()` を読んで期待の値と比べる。
    - 直さないと: 目に見える不透明度と見え隠れの同期に守りが無い。

41. **子の順と Z の順のテストは、WUC の子の並びを一度も見ない**（`crates/wintf/tests/visual/child_order_test.rs:L22-L229`・`hierarchy_sync_test.rs:L13-L55`・`L119-L220`）
    - 困ること: 確かめるのは bevy の `Children` の順と `parent_visual().is_some()` だけ。`InsertAtBottom` を `InsertAtTop` に変えても全部緑。`hierarchy_sync` の L119-L172 は順を変えず Z も見ない。L174-L220 は同期が変えられない `!is_valid()` を見る。
    - 直し方: 親の `ContainerVisual::Children()` を回して子の順を比べ、`is_some()` を「親と同じ」に変える。重なる L119-L172 は消し、L174-L220 は `parent_visual().is_none()` を確かめる。
    - 直さないと: 描く順が逆になっても通る。

42. **描画の部品が作られるかのテストが、テストが自分で入れた部品を確かめる**（`crates/wintf/tests/visual/graphics_auto_creation_test.rs:L51-L166`・`component_test.rs:L39-L80`）
    - 困ること: `VisualGraphics::default()` を入れてから `is_some()` を見る。L79-L99 は確かめがコメントで消されている。L101-L131 は「作り直さない」と言うが `is_valid` を 2 回見るだけ。作る呼び出しを消しても緑。
    - 直し方: `is_valid()` を確かめる・L94-L96 の確かめを戻す・前後で `as_raw()` を比べる。`component_test.rs:L39-L80` は消す。
    - 直さないと: 「GPU の見た目が作られるか」に本当のテストが無い。

43. **モニターの階層のテストは、既定値の部品が在るだけで「計算された」とみなす**（`crates/wintf/tests/window/monitor_hierarchy_test.rs:L260-L272`・`L394-L452`）
    - 困ること: `build_taffy_styles_system` が `TaffyComputedLayout::default()` を入れるので、在ることは何も示さない。L394-L452 は順の無い組で system を足し、`LayoutRoot` の下にも無いので計算されない。計算の system を消しても緑。
    - 直し方: L260-L272 は「既定値でない」を確かめる。L394-L452 は消す（L350-L392 が覆う）。
    - 直さないと: モニターの上のレイアウトの後退が通る。

44. **「ビットの一致」の検査が、同じ装置・同じ命令・同じ入れ物どうしを比べている**（`crates/wintf/tests/graphics/surface_pixel_equivalence_test.rs:L1-L397`）
    - 困ること: WUC の側も、テストが自分で作った入れ物へ同じ `draw_scene` を描く（L252-L288）。本番の描く係（`render.rs:95-111` の変形と `DrawImage`）は一度も走らず、WUC の描画面の中身も読まない。本番の変形を壊しても緑。
    - 直し方: ファイルを消す（`begin_draw`／`end_draw` の往復は 38 番を直した後の `surface_systems_test.rs` が通る）。
    - 直さないと: 約 400 行が画素の一致を偽って保証する。

45. **見た目のテストの実行ファイルが、GPU のテストの決まりが「落ちる」と書く別スレッドの Compositor の型を使う**（`crates/wintf/tests/visual/common/mod.rs:L23-L60`・`graphics_auto_creation_test.rs:L18-L49`・比べる先 `tests/graphics/common/mod.rs:L3-L18`）
    - 困ること: `tests/graphics/common` は「別スレッドに 2 つ目の MTA の Compositor を作ると STATUS_ACCESS_VIOLATION」を根本原因とし、1 本の持ち主のスレッドへ寄せた。見た目の実行ファイルは 27 本のテストがテストごとのスレッドで Compositor を作り、「慣らしの描画面」で安定させている（`on_gpu_owner_thread` は 0 件）。2 つの説明のどちらかが誤りで、別の GPU では実行ファイルごと落ちうる。
    - 直し方: `on_gpu_owner_thread` を `visual/common` へ写して 27 本を包み、慣らしの描画面を消す（完了した `wintf-gpu-test-crash` の design.md:122 が予定していた形）。
    - 直さないと: 確かめの失敗ではなくプロセスの死として落ちうる。

46. **マウスが離れた・ドラッグの守りのテストが、処理を呼ばず写しを試す**（`crates/wintf/tests/window/multiwindow_event_test.rs:L189-L307`）
    - 困ること: L221-L236 は `WM_MOUSELEAVE` のループの写し。L251-L307 は自分の注記（L258-L263）が「今の守りは `find_owner_window` を使わない」と言う＝消えた振る舞いのテストで、確かめは同語反復。
    - 直し方: 2 本とも消す（`find_owner_window` は L20-L112 が覆う）。
    - 直さないと: 覆っていない所が覆われて見える。

47. **「既に在る Visual を残す」のテストが上書きを見分けられない**（`crates/wintf/tests/visual/widget_visual_auto_insert_test.rs:L47-L70`）
    - 困ること: `is_visible` は残しても既定値で上書きしても真。`on_label_add` の早い戻りを消しても緑。
    - 直し方: `assert_eq!(visual.opacity, 0.5)`。
    - 直さないと: 部品の不透明度が黙って戻る後退が通る。

48. **装置のポインタの比べが、2 つの生きた局所の欄の番地を比べている**（`crates/wintf/tests/graphics/reinit_unit_test.rs:L140-L164`）
    - 困ること: `d2d_device()` は局所の `Option` の中の欄への参照を返す（`src/ecs/graphics/core.rs:81-83`）。2 つの別の局所なので `assert_ne` はいつも真。
    - 直し方: テストを消す（作り直しは `core_test.rs:L24-L35` が覆う）。
    - 直さないと: 同語反復が作り直しの確かめに見える。

#### テスト（`tests/layout`）

49. **不正な大きさを飛ばすかのテストが、試す system を一度も走らせていない**（`crates/wintf/tests/layout/graphics_sync_test.rs:L90-L137`・L124）
    - 困ること: `FrameCount` を入れず、`let _ = system.run(...)` が引数の検査の失敗を捨てる（34 番と同じ型）。`width <= 0 || height <= 0` の守りを消しても緑。
    - 直し方: `FrameCount(1)` を入れ、`run(...).unwrap()` にする。
    - 直さないと: 0x0 への `SetWindowPos` を止める守りに生きたテストが無い。

50. **帰還の輪の収束のテストが、同じ値の書き直しを見分けられない**（`crates/wintf/tests/layout/feedback_loop_convergence_test.rs:L78-L86`・`L307-L347`・`L353-L528`）
    - 困ること: 3 つとも値の一致だけを見る。「等しければ書かない」守り（`window_pos_systems.rs:159`）や `Changed<WindowPos>` の濾しを消しても、同じ値が書かれるだけなので緑。ファイルの名の要件（R1-AC1・R4-AC1）に失敗しうるテストが無い。
    - 直し方: 回す前後で `get_change_ticks` の刻みを比べて等しいことを確かめる。濾しのテストは、`WindowPos` を変えずに `Arrangement` を別の値にして、残ることを確かめる。
    - 直さないと: 毎フレームの余計な `Changed` の復活がテストを抜ける。

51. **flex のレイアウトのテストの後半は何も確かめず、注記の実測値も古い**（`crates/wintf/tests/layout/taffy_flex_layout_pure_test.rs:L118-L157`・`L312-L326`・`L195-L307`）
    - 困ること: 後半の `verify_changed_layout` は TODO で表示だけ。`if let Some` が部品の欠けを黙って飛ばす。注記の実測値（(400,0)・400x600）は確かめの値（(0,0)・800x600）と食い違う。2・3 番目の項目は大きさも位置も見ないので、grow 1 と 2 の取り違えを捕まえない。表示が約 40 行。
    - 直し方: 後半とその関数とヘッダの主張 3 を消す（再計算は `unit_integration.rs:L219-L433` が覆う）。古い注記と表示を消し、2・3 番目の幅と位置を確かめる。
    - 直さないと: 再計算と flex-grow を覆っているように見えて覆っていない。

52. **taffy の結合のテストに、失敗しえないものと他の重なりがまとまっている**（`crates/wintf/tests/layout/taffy_layout_integration_test/unit_integration.rs:L5-L76`・`L92-L208`・`L362-L418`）
    - 困ること: L147-L208 の「パラメーターの変更で再計算」は、新しい問い合わせが最初の挿入も「変わった」と数えるので、2 回目が何もしなくても通る（注記 L177 の「欄は private」も古い）。ほかは他のファイルとの重なり（L5-L21・L23-L66・L92-L115）・入れた部品を在ると確かめる（L117-L145）・失敗しえない確かめ（L409-L418）・表示（L362-L376・L393-L401）。
    - 直し方: 挙げた範囲を消す（L78-L90 と L219-L433 の残りは残す）。
    - 直さないと: 約 150 行が覆っているように見える。

53. **木の同期の差分のテストが、テストが自分で入れた部品を確かめ、漏れのテストは漏れを見ない**（`crates/wintf/tests/layout/taffy_advanced_test/tree_sync.rs:L133-L173`・`L233-L278`・`L282-L370`）
    - 困ること: 2 本は自分で入れた `TaffyComputedLayout` が在ることを見る（いつも真）・system を `.chain()` なしで足す。「漏れを防ぐ」テストは 2 つの表の長さが揃うかだけを見る（10 件残っても 10＝10 で通る）うえ、確かめは `#[cfg(debug_assertions)]` の中だけ＝release では何も確かめない。L282-L342 は 1 つの場面を 2 本に割る。
    - 直し方: 変更の刻みを比べる・`.chain()` を足して子の節の並びを確かめる・10 件の実体それぞれの `get_node` が `None` を cfg なしで確かめる・2 本を 1 本にまとめる。
    - 直さないと: 差分の計算と後片づけの漏れにテストが無い。

54. **窓の座標の変換のテストが、何も変換しなくても通り、しかも本物の窓を画面に出す**（`crates/wintf/tests/layout/client_area_positioning_test.rs:L16`・`L47-L65`・`L73`）
    - 困ること: 確かめは `<=`・`>=` で、入力をそのまま返しても通る。`WS_VISIBLE` で 400x300 の窓を出して前面を奪い、同じ机で走る `SendInput` の実機テストを揺らしうる（読む API は見えない窓でも動く）。
    - 直し方: `<`・`>` にする。`WS_VISIBLE` を外す。
    - 直さないと: 枠のある窓の変換の後退を捕まえず、画面を荒らす。

55. **`drag_lifecycle.rs` はドラッグのテストの写しで、入口の設定を 5 回貼っている**（`crates/wintf/tests/layout/boxstyle_coordinate_separation_test/drag_lifecycle.rs:L1-L398`・比べる先 `tests/drag/dispatch_test.rs:L50-L237`）
    - 困ること: 5 本は `dispatch_test.rs` と同じ地面を覆う。中の通しのテスト（L332-L398）は 2 本の足し算。ほかに無い確かめは `initial_window_pos` の 1 つだけ。`if let Some(ctx)` は鍵が毒されたとき黙って通る。
    - 直し方: ファイルと `mod.rs:L14` を消し、`initial_window_pos` の確かめを `dispatch_test.rs` へ移す。
    - 直さないと: ドラッグの口が変わるたびに 400 行を 2 度直す。

56. **絶対位置の変換のテストが `inset` を一度も確かめない**（`crates/wintf/tests/layout/box_style_consolidation_test.rs:L213-L232`）
    - 困ること: `inset` を設定するが `position` しか見ない。テスト全体で `inset` の値を確かめる所が無い。レイアウトの根は `inset` で仮想の画面の原点に置かれる（`monitor_systems.rs:96-101`）ので、変換を落とすと全部の窓がモニターから外れて、なお全部緑。
    - 直し方: `inset.left`・`inset.top` の 2 行の確かめを足す。
    - 直さないと: 窓の置き場所の要の変換に守りが無い。

57. **`graphics_sync_test.rs` の 4 本は wintf の論理を試さないか、重なっている**（`crates/wintf/tests/layout/graphics_sync_test.rs:L78-L88`・`L139-L296`）
    - 困ること: 同じ確かめを 2 度する・別のテストの写し＋bevy の `bypass_change_detection` の往復・bevy の `get_mut` の往復だけ・derive の `PartialEq` だけ。L263-L296 の注記（「Mut を落とすとき bevy が PartialEq で比べる」）は誤り。
    - 直し方: 4 本を消す。
    - 直さないと: 約 140 行の失敗しえないテストと、変更の検知についての誤った注記が残る。

58. **境目の場面のテストが「落ちない」しか示さず、在るかの確かめは失敗しえない**（`crates/wintf/tests/layout/taffy_advanced_test/boundary_scenarios.rs:L75-L256`）
    - 困ること: 自分で入れた部品やフックが入れる部品の在る無しを見る。`.chain()` が無いので計算の前に配置が走りうる。0x0 は既定値と同じなので計算しなくても通る。
    - 直し方: `.chain()` を足し、計算が済んで初めて出る値（子 100 個・最も深い節の親など）を 1 つずつ確かめる。
    - 直さないと: 境目のテストがレイアウトの後退を捕まえない。

59. **`hierarchical_bounds_test.rs` は `arrangement.rs` の同じ名のテストを弱く写している**（`crates/wintf/tests/layout/hierarchical_bounds_test.rs:L12-L29`・`L82-L144`）
    - 困ること: 同じ名・同じ題・同じ入力と期待の値（(10,20) を 2 倍・100x50→(20,40,220,140)）。こちらは単位の変形だけで弱い。
    - 直し方: L82-L144 と L22-L29 を消す。
    - 直さないと: 1 つの振る舞いが同じ名の 2 本に分かれて住む。

#### example（`examples/`・`Cargo.toml` の見出し）

60. **使われない 228 KB の画像が crates.io の荷に入る**（`crates/wintf/examples/dcomp_demo.jpg`）
    - 困ること: 参照 0 件（`git grep` 0）。`Cargo.toml` に `exclude` が無いので、出すたびに荷へ入る。
    - 直し方: 消す。
    - 直さないと: どの版も 228 KB 重い。

61. **`wuc_spike.rs` は終わった移行の試し掘りで、終了の待ちの写しと誤った注記を持つ**（`crates/wintf/examples/wuc_spike.rs:L1-L302`・写し L170-L193・注記 L208 と L226-L231）
    - 困ること: 問い（DComp→WUC の可否）は決着し、本番が答えを使っている（`wuc_resource.rs:73-98`・`runtime/mod.rs:158`）。L170-L193 は 12 番の関数の写し。L208 は「全面のクリア＋中央の矩形」と言うが、クリアしかしない。「spike」の名と内部の要件番号のまま利用者へ出ている。
    - 直し方: ファイルを消す（本番の 3 か所の由来のコメントは「R1 の試し掘り（削除済み）」に直す）。
    - 直さないと: 終わった実験 300 行が出荷され、写しが本物とずれていく。

62. **`dcomp_taffy_demo.rs` は `taffy_flex_demo` の 1 行目の写し**（`crates/wintf/examples/dcomp_taffy_demo.rs:L1-L348`・比べる先 `taffy_flex_demo/setup.rs:L40-L226`）
    - 困ること: 描く道が 2 つあった頃の比べで、今は WUC の 1 本。同じ箱・同じ画像。足すのは `Label` だけで、ほかの example が覆う。
    - 直し方: 消す。
    - 直さないと: 348 行の写しが、古い DComp の文言ごと出荷される。

63. **`taffy_flex_demo` の約 350 行が `#[allow(dead_code)]` に隠れた死んだコード**（`crates/wintf/examples/taffy_flex_demo/diagnostics.rs:L201-L550`・`main.rs:L88`・`L115-L169`・`setup.rs` の各 spawn）
    - 困ること: 当たり判定の診断の 3 関数と、それだけが呼ぶ 2 関数は呼ばれない。期待の値（600x400・倍率 1.25）は今の demo に無い場面。消すと 14 個の印の部品が生成されるだけで読まれなくなる（5 個は今も読まれない）。
    - 直し方: 死んだ関数・印の型・`hit_test` の再公開・spawn の印を消す（`Name` で見分けられる）。
    - 直さないと: 古い座標の死んだコードが、蘇らせる人を誤らせる。

64. **`graphics_reinit_test` は、何も作り直されていなくても「成功」と出す**（`crates/wintf/examples/graphics_reinit_test.rs:L118-L148`）
    - 困ること: 世代 0 の実体は「待ち」の枝で `all_success` を落とさないので、作り直しの鎖が壊れても成功と出る。対象が 0 件でも成功。ログの購読者を立てないので、本体の作り直しのログが出ない。世代の上がりを通しで確かめる所はこれだけ（`tests/` に 0 件）。
    - 直し方: 数えた件数が 1 以上・世代 0 の枝で失敗・ほかの example と同じ 4 行の `tracing_subscriber` を足す。
    - 直さないと: 手で確かめる人が、壊れた作り直しに緑を見る。

65. **steering が RUST_LOG の濾しの例に `multi_window_test` を挙げるが、この example はログを立てない**（`crates/wintf/examples/multi_window_test.rs:L9-L15`・`L89-L111`・範囲の外 `.kiro/steering/logging.md:L118`）
    - 困ること: `logging.md:118` のコマンドを打っても wintf のログは 1 行も出ない。release の窓の型ではコンソールが無く、`read_line` はすぐ戻る。失敗の道は `Ok(())` で終了コード 0。中身は 64 番の部分集合。
    - 直し方: ファイルを消し、`logging.md:118` をログを立てる example（`clip_demo` など）へ向け直す。
    - 直さないと: steering の手順が壊れたまま、似た「_test」の example が 2 つ残る。

66. **example とクレートの見出しが、まだ「DComp／DirectComposition」と言う**（`crates/wintf/examples/clip_demo.rs:L5-L335` の各所・`dcomp_demo.rs`（ファイル名と L3-L118）・`dcomp_taffy_demo.rs:L3-L316`・`crates/wintf/Cargo.toml:L11` の keywords `"directcomposition"`）〔公開（crates.io の見出し）〕
    - 困ること: 窓の題「Clip Demo (DComp)」や画面の文「DirectComposition で描画」は、今の WUC と違う。`clip_demo` の L274-L277 で手で足す `WS_EX_NOREDIRECTIONBITMAP` は本体がいつも足すので効かない。crates.io の検索の語も古い。
    - 直し方: 62 番で 1 本消し、`clip_demo` の文言と名前を WUC に直して効かない行を消す。`dcomp_demo` は 112 番。keywords の語を外す（または `"windows-ui-composition"` 系へ）。
    - 直さないと: 公開の example を読む人が、まだ DirectComposition を使っていると思う。

67. **`typewriter_demo` は完了の出来事を見せるとうたうが、誰も知らせない**（`crates/wintf/examples/typewriter_demo.rs:L9`・`L41-L43`・`L182`・`L211-L213`・`L252`・`L329-L334`・`L368-L371`）
    - 困ること: 本体は受け手に `TypewriterEvent::Complete` を入れるだけでログを出さず、example にも読む system が無い。ほかに注記の食い違い（横書きに空の会話を入れていない・「薄い青」「薄い緑」が実際は黒・「固定の大きさ」が実際は flex_grow）。
    - 直し方: 受け手の `Changed<TypewriterEvent>` を `info!` する system を 1 本足す（または主張と受け手を消す）。注記を直す。
    - 直さないと: doc が並べる機能のうち 1 つだけ確かめられない。

68. **`taffy_flex_demo` は窓 1 を決め打ちの (-1600, 400) へ動かし「別のモニター」と呼ぶ**（`crates/wintf/examples/taffy_flex_demo/main.rs:L314-L330`・`L200`・`L315`）
    - 困ること: 左にモニターが無い机では、2 秒で窓が画面の外へ出て、残り 62 秒の場面を試せない。同じファイルに `find_non_primary_monitor_origin()`（L213-L261）がある。L315 の `#[allow(dead_code)]` は呼ばれている関数に付いている。
    - 直し方: その関数が見つけたときだけ動かし、無ければ 1 行知らせる。古い allow を消す。
    - 直さないと: モニター 1 枚の机では demo の半分が使えない。

---

### できれば（Nice to have）

#### 本番のコード

69. **テストの外に呼び手の無い COM の拡張メソッド**（`crates/wintf/src/com/d2d/mod.rs:L20-L43`（`D2D1FactoryExt` の 2 つ）・`L51-L70`（`create_command_list`＝呼ぶたびに装置の文脈を作って捨てる）・`L97-L99`／`L170-L175`・`L111-L122`／`L196-L230`・`L157-L165`／`L289-L312`・`L94`（`#[delegatable_trait]`）・`d3d11.rs:L44-L78`・`dwrite.rs:L92-L139`・`L173-L205`）〔公開〕
    - 困ること: `create_path_geometry`・`create_rounded_rectangle_geometry`・`create_command_list`・`create_bitmap_from_wic_bitmap`・`draw_text`・`fill_geometry`・`draw_image`・`get_device_removed_reason`・`create_texture2d`・`set_text_alignment`・`set_paragraph_alignment`・`get_cluster_count`・`hit_test_text_position` は `tests/com/` の外の呼び手が 0（`git grep -w`・`draw_image` の 1 件はコメント）。
    - 直し方: それぞれと、それだけのためのテストを消す。
    - 直さないと: 使い手の無い公開の面を保ち続ける。

70. **注記が存在しない `com/dcomp.rs` を引く**（`crates/wintf/src/com/wuc.rs:L6-L11`・`L186-L189`・`L198`）
    - 直し方: dcomp の文を消し、何を返すかだけを残す。

71. **`MessageLoopDriver::run` と `default_filter` は死んだコード**（`crates/wintf/src/runtime/message_loop.rs:L34-L40`・`L59-L80`・テスト `L203-L213`）
    - 困ること: `#[allow(dead_code)] // 本番の呼び手は無い` は使わない理由で、残す理由ではない（先送りの印に当たらない）。呼び手 0。
    - 直し方: 2 つの関数とテストと doc の項目と、要らなくなる import を消す。

72. **run の最後の「尾の競り合い」の起こしは誰も起こさず、使う関数は 1 行の包み**（`crates/wintf/src/runtime/mod.rs:L347-L348`・`L439-L440`・`message_loop.rs:L154-L156`・`L180-L188`）
    - 困ること: 終了の合図を待つ本番の唯一の聞き手は、`block_on` が戻った時にはもう終わっている。単一スレッドで、聞き手は確かめる前に構えるので、書いてある競り合いは起きない。
    - 直し方: L439-L440 と注記を消し、`notify_shutdown` を `request_exit` の中へ畳む。

73. **呼びを素通しするだけの名前の箱 2 つ**（`crates/wintf/src/runtime/message_loop.rs:L41-L57`（`MessageLoopDriver::block_on`）・`tick_bridge.rs:L167-L187`（`AsyncTickTask::spawn`））
    - 直し方: `crate::executor::block_on` と `spawn_local(run_async_tick(..))` を直接呼び、2 つの構造体を消す。

74. **`WinApp::spawn_ui_local` は `self` を使わない公開の包み**（`crates/wintf/src/runtime/mod.rs:L446-L455`）〔公開〕
    - 困ること: `wintf::executor::spawn_local`（`lib.rs:L11`）が既に公開。呼び手は `examples/dcomp_demo.rs:L76` だけ。
    - 直し方: 消して example を `wintf::executor::spawn_local` に変える。

75. **doc は窓の生成に `new_checked_ex` を使うと言うが、コードは `new_ex`**（`crates/wintf/src/runtime/mod.rs:L62-L64`・`window_factory.rs:L90`・`L95-L97`・`L418`）
    - 困ること: 再入の扱いが逆に書いてある（実際は World を借りたまま `new_ex` で作り、生成時のメッセージは借用中の道で飛ばす）。
    - 直し方: doc を実際に合わせる。

76. **`wire_new_path` の doc が「冪等」と言い、同じ doc が冪等でないと言う**（`crates/wintf/src/runtime/mod.rs:L232-L234` と `L251-L254`）
    - 困ること: 要約を信じて `run()` を 2 度呼ぶと、2 度目の最初の刻みで引数の検査に落ちる。
    - 直し方: 冪等の語を消す。

77. **「後のタスクで結線する」という古い注記**（`crates/wintf/src/runtime/mod.rs:L117-L119`・`message_loop.rs:L5-L7`・`L35`・`tick_bridge.rs:L5-L6`・`L18-L21`・`L164-L166`・`window_registry.rs:L20-L25`・`L36-L39`・`L69-L72`・`window_factory.rs:L82-L84`）
    - 直し方: その文を消す（結線は済んでいる。合図の持ち主は `AppExit`）。

78. **doc は刻みと中継のタスクが自分で終わると言うが、正常な終了の後は二度と起きない**（`crates/wintf/src/runtime/mod.rs:L409-L411`・`tick_bridge.rs:L173-L174`・`L278-L279`）
    - 困ること: run の後は vblank の起こしが来ないので、2 つのタスクは止まったまま（run 1 回につき 1 度の小さな残り・本番では害なし）。
    - 直し方: doc を実際に合わせる。

79. **`event_listener` のクレートだけを試すテスト 2 本**（`crates/wintf/src/runtime/message_loop.rs:L224-L255`）
    - 困ること: `shutdown_future` を呼ばない。本当の網羅は L316-L371。
    - 直し方: 2 本を消す。

80. **テストの中の「将来のための」import と黙らせの行**（`crates/wintf/src/runtime/window_factory.rs:L291-L294`・`L380`・`L404-L405`・`L490`）
    - 直し方: 2 つの import と `let _ = (...)`・`let _ = registry;` を消す。

81. **テストの doc は窓が見えないと言うが、生成は窓を出す**（`crates/wintf/src/runtime/mod.rs:L573-L574`・`window_factory.rs:L332-L336`・`L423-L424`）
    - 困ること: `create_window` はいつも `ShowWindow(SW_SHOW)` を呼ぶ（L184-L186）。4 本のテストが本物の透明な窓を画面に出し、前面を奪いうる。
    - 直し方: doc を直す。前面を奪うのが問題なら、テストで確かめる前に隠す。

82. **ずれた引き先と誤った「本番と同じ」の主張（重なりの順の調べのテスト）**（`crates/wintf/src/api_owner_chain_probe_tests.rs:L15-L20`・`L32-L35`・`L57`・`L80`・`L124`・`L306-L313`・`L598`・`L896-L898`・`api_owner_chain_nudge_probe_tests.rs:L53-L56`）
    - 困ること: 本番の窓の型の引き先（`placement/spawn.rs:572-576`）は今はコメント（本物は 625 行・調べの窓は `WS_EX_TOOLWINDOW` だけ）。存在しない「実測 8」を引く。`zorder_pair_maintain.rs:286` は今 292 行。採った形の説明が古い。空の見出し。
    - 直し方: 行を直すか消す（コードは変えない）。

83. **調べのテストの素通しの補助**（`crates/wintf/src/api_owner_chain_probe_tests.rs:L193-L196`（`is_visible`＝`api.rs:L112` の写し）・`L302-L304`（`place_at`＝`place_after` の素通し・8 か所））
    - 直し方: 本物を呼び、2 つを消す。

84. **試験の窓の作り方が 19 ファイルに貼られている**（範囲の中: `crates/wintf/src/win_style.rs:L673-L761`・`L773-L850`・`win_style_accept_files_tests.rs:L14-L72`・`api.rs:L370-L399`・`api_owner_chain_probe_tests.rs:L83-L107`）〔範囲の外（置き場所 `src/ecs/test_support.rs`）〕
    - 困ること: `CreateWindowExW("Static")`＋`catch_unwind`＋`DestroyWindow` の同じ形（`git grep -l 'w!("Static")' crates/wintf/src`＝19 ファイル）。
    - 直し方: `with_test_window(ex_style, style, parent, |hwnd| ..)` を 1 つ作って全部で使う。置き場所の `ecs/test_support.rs` は範囲の外。18 番で `win_style.rs` のテストの多くが消える。

85. **`Cargo.toml` の注記の数が古い**（`crates/wintf/Cargo.toml:L49`「消費 10 crate」）
    - 困ること: 実際は 18。
    - 直し方: 数を外す。

86. **`aabb.rs` の重なった import**（`crates/wintf/src/numerics/aabb.rs:L169`）〔描画の木と同じファイル〕
    - 困ること: L4 の `use core::ops::*` が既に覆う。
    - 直し方: L169 を消す（import の行だけで、働きは消さない）。

87. **`clear_window_owner` の doc が、持ち主の無い窓で偽の失敗が返ることを言わない**（`crates/wintf/src/api.rs:L145-L154`）
    - 困ること: 調べ 5（`api_owner_chain_probe_tests.rs:L719-L750`）が `Err` を固定し「偽の失敗」と呼ぶが、doc は失敗を本物として読ませる。
    - 直し方: 「owner を持たない窓では OS が偽の失敗を返す。呼ぶ前に owner の有無を読むこと」の 1 行を足す。

88. **数の型の使われない働き**（`crates/wintf/src/numerics/aabb.rs`（`contains`・`intersect`・`union`・`translate`・演算子の実装の多く）・`point2.rs`（numerics の外で 0）・`matrix3x2.rs:L14-L24`（`moved`＝名は `eps_px` だが単位の正方形の角しか比べない・利用 0）・`transform2d.rs`（`pivot_srt`・`to_affine` の呼び手 0））〔描画の木 申し送り〕〔残す候補 ⑵〕
    - 困ること: 範囲の中でも外でも呼び手が 0。ただし描画の木を本番へ上げる部品なので、brief の決まりにより未使用を理由に消さない。`moved` の単位の誤り（1.001 倍の拡大で 1000 px の矩形は 1 px 動くのに「動かない」と答える）は直す価値がある。
    - 直し方: この spec では消さず、`wintf-visual-tree` へ申し送る（使い手の表は下の 4 節）。

#### テスト

89. **読まれないテストの資産 6 つと、それを書く生成の道具**（`crates/wintf/tests/assets/seikatu_0_1.webp`・`seikatu_0_2.webp`・`seikatu_1_0.webp`・`seikatu_1_1.webp`・`seikatu_1_2.webp`・`demo_checker_64x64.png`・`examples/generate_test_image.rs:L13-L22`・`L65-L88`・`examples/split_image.rs:L1-L42`）
    - 困ること: 6 つは参照 0 件（書く道具だけが名を持つ）。`seikatu.webp` を読むのは `split_image` だけ。生成の道具は管理下のソースの木へ書く（意図どおり）が、crates.io から取った荷で走らせると登録簿の中へ書く。
    - 直し方: 6 つを消す。`generate_test_image` の格子の部分を消し、`split_image.rs` と `seikatu.webp` を消す（`seikatu_0_0.webp` と色の地図の png は example が読むので残す）。

90. **同じ 2 つの補助が 2 か所にあり、効かない `#[allow(dead_code)]` が付く**（`crates/wintf/tests/com/common/mod.rs:L6-L28`・`tests/widget/bitmap_source_integration_test.rs:L14-L35`）
    - 直し方: `tests/widget.rs` に `#[path = "com/common/mod.rs"] mod common;` を足して写しを消す。L7・L18 の allow を消す。

91. **同じファイルの通しのテストが繰り返す煙のテスト**（`crates/wintf/tests/com/animation_test.rs:L41-L74`・`d3d11_test.rs:L216-L219`・`wic_test.rs:L223-L228`・`d2d_ext_test.rs:L72-L81`・`drag/window_dragging_filter_test.rs:L297-L379`）
    - 直し方: 繰り返しを消す（`animation_test.rs` は 14 番で丸ごと消える）。

92. **doc が確かめる以上のことを言う**（`crates/wintf/tests/ecs/tree_propagation_test.rs:L405-L435`（「静かな部分木を飛ばす」を言うが、結果が安定することしか示せない）・`tests/com/dwrite_test.rs:L156-L165`（「切り詰める」と言うが `is_ok()` だけ））
    - 直し方: doc を「冪等」に直す。dwrite の方は消すか `textPosition` を確かめる。

93. **名前を変えた包みの型の、失敗しえないテスト**（`crates/wintf/tests/layout/taffy_layout_integration_test/wrapper_types.rs:L1-L89`）
    - 困ること: 入れて在る・既定値どうしの比べ・derive・`size_of`（`repr(transparent)` の有無を見分けられない）。
    - 直し方: ファイルと `mod wrapper_types;` を消す。

94. **`box_components.rs` は入れた値を読み戻し、使い手 0 の型の既定値を固定する**（`crates/wintf/tests/layout/taffy_layout_integration_test/box_components.rs:L1-L147`）〔範囲の外（`FlexContainer`／`FlexItem` は `src/ecs/layout/box_style.rs`）〕
    - 直し方: ファイルと `mod box_components;` を消す。使い手 0 の `FlexContainer`／`FlexItem` は `audit-wintf-layout-widget` の範囲。

95. **`box_style_consolidation_test.rs` の失敗しえないテストと重なり**（`crates/wintf/tests/layout/box_style_consolidation_test.rs:L10-L19`・`L45-L128`・`L181-L211`・`L234-L263`）
    - 直し方: 挙げた範囲を消す（56 番の確かめは残す）。

96. **大きさの変化のテストは bevy だけを試す**（`crates/wintf/tests/layout/boxstyle_coordinate_separation_test/changed_timing.rs:L80-L148`）
    - 直し方: 消す。

97. **`boxstyle_inset.rs` は帰還の輪のテストの写し**（`crates/wintf/tests/layout/boxstyle_coordinate_separation_test/boxstyle_inset.rs:L12-L113`）
    - 直し方: `inset.is_none()` の 1 行を `feedback_loop_convergence_test.rs` の L24 のテストへ移し、ファイルと `mod boxstyle_inset;` を消す。

98. **帰還の輪とレイアウトの変換の小さな重なり**（`crates/wintf/tests/layout/feedback_loop_convergence_test.rs:L160-L204`・`taffy_advanced_test/layout_conversion.rs:L9-L25`）
    - 直し方: 2 か所を消す。

99. **帰還の輪のテストの中で組み立てを 3 回・窓の生成を 6 回貼っている**（`crates/wintf/tests/layout/feedback_loop_convergence_test.rs:L55-L62`・`L386-L393`・`L490-L496`）
    - 困ること: 本番は `.chain()`（`src/ecs/world/mod.rs:279-290`）で、手の写しとずれうる。
    - 直し方: このファイルの中に `post_layout_schedule()` と `spawn_window(..)` を 1 つずつ置く。

100. **子の順のテストの補助の doc が回数を誤り、ほぼ同じテストがもう 1 本ある**（`crates/wintf/tests/layout/taffy_child_order_test.rs:L22-L27`・`L99-L141`）
     - 直し方: doc を直す。L99-L141 を消す。

101. **寸法の型のテストの煙と、本番の使い手 0 の非推奨の型のテスト**（`crates/wintf/tests/layout/metrics_test.rs:L11`・`L32-L50`・`L52-L91`・`L105-L122`）〔範囲の外（`Opacity` は `src/ecs/layout/metrics.rs`）〕〔公開〕
     - 困ること: 確かめの無い呼び出し・署名で保証されること・derive の確かめ。`Opacity` は `#[deprecated]` で本番の使い手 0（ヘッダの「利用箇所がある」は誤り）。
     - 直し方: 煙と derive のテストを消す。`Opacity` のテストは型を消すときにいっしょに消す（型は `audit-wintf-layout-widget` の範囲）。

102. **配置のテストのデバッグの表示と derive だけのテスト**（`crates/wintf/tests/layout/arrangement_bounds_test/arrangement.rs:L13-L24`・`L98-L111`）
     - 直し方: 2 か所を消す。

103. **空の登録簿で「整合」を確かめるテスト**（`crates/wintf/tests/layout/taffy_advanced_test/mapping_management.rs:L71-L78`）
     - 直し方: `unit_integration.rs:L54-L66` と 1 本にし、作って消した後に確かめる。

104. **bevy 自身を試すテストと表示の雑音**（`crates/wintf/tests/graphics/core_ecs_test.rs:L41`・`L74`・`L106`・`L109-L261`）
     - 直し方: リソースの入れ・読みだけのテスト 6 本と表示を消す。

105. **ビルドの種類ごとの重なったテスト**（`crates/wintf/tests/graphics/core_test.rs:L21`・`L34`・`L51`・`L65`・`L68-L88`）
     - 直し方: `test_debug_build`／`test_release_build` と表示を消す。

106. **`frame_time_test.rs` は入れた値を読み戻すだけで、要らない 1ms の眠りと時計を持つ**（`crates/wintf/tests/graphics/frame_time_test.rs:L1-L59`）
     - 直し方: ファイルと宣言を消す。

107. **描画面の最適化のテストの同語反復**（`crates/wintf/tests/graphics/surface_optimization_test.rs:L64-L88`・`L130-L147`）
     - 直し方: 2 本を消す（19 番で `bevy_app` を開発用へ移すとき、このファイルの `App` の利用も見直す）。

108. **縦書きのレイアウトのテストが縦書きのレイアウトを試さない**（`crates/wintf/tests/widget/vertical_text_layout_test.rs:L1-L34`）
     - 直し方: 3 本（`x == x`・欄の読み戻し 2 本）と要らない `#[cfg(test)] mod tests` を消し、L36-L53 を残す。

109. **`parent_visual_test.rs` は `components_test` の写し**（`crates/wintf/tests/visual/parent_visual_test.rs:L31-L163`）
     - 直し方: ファイルと `tests/visual.rs` の宣言を消す。

110. **`remove_visual_api_test.rs` は wintf でなく Microsoft の API を試す**（`crates/wintf/tests/visual/remove_visual_api_test.rs:L1-L135`）
     - 困ること: 試していた wintf の口は消えた（doc L3-L5）。テストの中の補助だけを呼び、L64-L82 は表示だけ。
     - 直し方: ファイルを消す。

111. **写された補助（`add_child`／`remove_child`・`init_com_mta`）**（`crates/wintf/tests/graphics/components_test.rs:L42-L57`・`visual/parent_visual_test.rs:L14-L29`・`visual/remove_visual_api_test.rs:L14-L29`・`visual/hierarchy_reparent_gap_test.rs:L20-L26`・`visual/common/mod.rs:L17-L21`・`graphics/reinit_unit_test.rs:L9-L15`・`graphics/surface_pixel_equivalence_test.rs:L57-L61`）
     - 直し方: 109・110・44 番で 3 つが消える。残りは各実行ファイルの `common` へ寄せる。

112. **`dcomp_demo.rs` は `spawn_ui_local` の 3 行の見本のほかに足すものが無い**（`crates/wintf/examples/dcomp_demo.rs:L1-L210`・独自の部分 `L74-L78`）
     - 困ること: 矩形と文字の格子は `clip_demo`・`tooltip_demo` が覆う。名前の「dcomp」は古い（66 番）。
     - 直し方: 74 番を採るならその行は不要、採らないなら L74-L78 を `clip_demo` へ移す。ファイルを消す。

113. **GPU の持ち主のスレッドの中の重ねた COM の初期化と、要らない包み**（`crates/wintf/tests/graphics/clip_sync_system_test.rs:22`・`components_test.rs:25`・`dcomp_integration_test.rs:20`・`init_window_graphics_test.rs:25`・`reinit_unit_test.rs:13`・`surface_pixel_equivalence_test.rs:59`・`surface_systems_test.rs:33`・`window_pos_systems_test.rs:102`・包みの要らない 7 本 `components_test.rs:L89-L97`・`reinit_unit_test.rs:L17-L50`・`L140-L164`・`window_pos_systems_test.rs:L59-L83`・`L169-L190`・`L196-L269`）
     - 困ること: 持ち主のスレッドは既に MTA で初期化する（`graphics/common/mod.rs:44-46`）ので、対の無い数を増やすだけ。Compositor を作らないテストまで包み、`common/mod.rs:16-18` の決まりに反して直列にする。
     - 直し方: 包みの中の `CoInitializeEx` を消し、7 本から包みを外す。

114. **試験の土台の自己テストが回帰の檻と重なる**（`crates/wintf/tests/graphics/common/mod.rs:L119-L131`・比べる先 `wuc_restart_regression_test.rs:L58-L81`）
     - 直し方: 重なる 2 本を消し、`owner_thread_returns_value` を残す。

115. **`WucVisualFactory::new` は引数を捨て、2 つ目の装置を作る**（`crates/wintf/tests/visual/common/mod.rs:L41-L45`）
     - 困ること: 見た目のテストのたびに D3D の装置を 2 つ作る。
     - 直し方: 引数の `d2d_device()` を使い、中の `GraphicsCore::new()` を消す。

116. **小さな重なりと同語反復**（`crates/wintf/tests/window/monitor_hierarchy_test.rs:L113-L202`・`L271`・`visual/component_test.rs:L8-L37`・`widget/bitmap_source_integration_test.rs:L205-L242`・`visual/insert_visual_test.rs:L83-L89`・`graphics/dcomp_integration_test.rs:L38-L91`）
     - 困ること: 親子の確かめの繰り返し（モニター 0 枚では空回り）と表示・構造体の読み戻し・windows-rs の口で偽になりえない `!is_null()`・届かない早い `return`・ECS のフックだけを試すのに WUC を丸ごと組む。
     - 直し方: L113-L202 を L65-L111 へ畳んで表示を消す・重なりを消す・`!is_null()` のテストを消す・`.unwrap()` にする・素の `World` にする。

#### example

117. **example の注記と画面の文がコードと合わない**（`crates/wintf/examples/taffy_flex_demo/main.rs:L32-L34`・`L192-L201`・`L265`・`L309`・`setup.rs:L298`・`L343`・`L489`・`L512`・`L576-L577`・`drag.rs:L89-L91`・`L115-L116`・`handlers.rs:L333`・`postmessage_click_test.rs:L121`）
     - 困ること: 領域の名（実際は 4 つの角）・閉じる時刻（4 秒と言うが約 64 秒）・ログの順・図形（四角を三角と言う）・色（黄は赤・水色は青）・「記録する」が実際はログだけ・10 フレームごとが実際は 30・「(20,20) ずらす」が実際は (0,0)。
     - 直し方: 文と注記を直す。閉じる時刻はログを待ちの後へ移すか、60 秒の待ちを外す。

118. **`graphics_reinit_test` は共有の指令の道を使わず、自前の通り道を持つ**（`crates/wintf/examples/graphics_reinit_test.rs:L26`・`L36-L40`・`L208-L219`）
     - 直し方: ほかの example と同じ `spawn(|tx| async move { .. Timer .. })` の形にして、自前の約 15 行を消す。

119. **`wuc_spike` の中の手での資源の漏れは、書いてある判断**（`crates/wintf/examples/wuc_spike.rs:L73-L75`・`L241`）〔残す候補 ⑵〕
     - 何か: 窓の組の名と題の入れ物を、試し掘りの寿命の間だけわざと漏らす（「一過性ゆえ leak 許容」）・`CoInitializeEx` の失敗の許容。
     - 直し方: 61 番でファイルを消せば消える。残すなら判断のまま。

---

### 判定と減らせる量

- **判定**: 本番の道そのものは概ね健全。先に直すのは 2 番（`DwmFlush` の失敗で刻みが止まる・相手の spec と調整）・4 番（文字の塊の失敗を黙って空にする）・3 番（位置だけの窓が跳ぶ）・1＋13 番（使えない unsafe な公開の口を消す）。テストは「失敗しえない」ものが多く、直すと今の本番の後退が見えてくる見込み。
- **減らせる量（見立て）**: 約 -7,500 行（本番 約 -2,400・example 約 -1,400・テスト 約 -3,700）・依存 -3（`nonmax`・`ambassador`・開発用の `rand`。`bevy_app` は開発用へ移す）・資産 -8（jpg 1・png 1・webp 6）。

---

## 4. 描画の木への申し送り（要件 3.4・3.5）

`crates/wintf/src/numerics/` の働きの使い手（範囲の外を含めて `git grep` で数えた）。この spec では、未使用を理由に消さない。

| 働き | 使い手 |
|---|---|
| `Aabb`（型・`new`・`left`/`top`/`right`/`bottom`） | `ecs/visual/clip.rs`・`draw/builder.rs`（14）・`draw/command.rs`・`draw_content.rs`・`world_visual.rs` |
| `Aabb::new_rect`・`width`/`height` | `draw/builder.rs` |
| `Aabb::EMPTY` | `world_visual.rs` |
| `Aabb` の `Hash`／`CalcHash`・`From<D2D_RECT_F>` | `draw_content.rs`・`clip.rs` |
| `Mul<Matrix3x2>`→`Aabb::transform` | `clip.rs:60` だけ |
| `Aabb::contains`／`intersect`／`union`／`translate`・演算子の多く | 0（描画の木は `intersect` を自前で写す＝22 番） |
| `Point2` | numerics の外で 0 |
| `Matrix3x2Ext::axis_aligned` | `world_visual.rs:52` |
| `Matrix3x2Ext::moved` | 0（88 番・単位の誤り） |
| `Transform2D` | `ecs/visual/transform.rs` |
| `Transform2D::pivot_srt`／`to_affine` | 0 |
| `CalcHash`（`pub(crate)`） | `draw/builder.rs:250`・`world_visual.rs:23` |
| `windows_numerics::*`・`D2D_RECT_F` の再公開 | `ecs/visual/` の 6 ファイル（`wintf::numerics::` の外からの利用 0） |

- 「直すべき傷」との重なり: 傷 3＝21 番・傷 4＝22 番。傷 1（COM の番地の hash）は範囲の外（`ecs/visual/draw_content.rs:22`・`builder.rs:249`）で、範囲の中の `numerics/calc_hash.rs` は小数だけを ±0 をそろえて hash する正しい作り（指摘なし）。傷 2・5〜8 は範囲の外。
- 申し送る指摘: 21・22・23・88 番（4 件）。

## 5. ほかの spec と重なる所（要件 3.6・6）

| 番号 | 相手 | 重なり |
|---|---|---|
| 2・8 | `wintf-dwmflush-wait-compensation` | 2 番はあちらの直す中身そのもの（空回り・`Err` の回に起こさない・注記の食い違い）。8 番は同じファイルの 1 行 |
| 5 | `audit-wintf-layout-widget` | 本番の直しは `src/ecs/layout/dimension.rs` |
| 30 | `audit-wintf-graphics-world` | 単一スレッドの名札の定数は `src/ecs/world/mod.rs` |
| 35 | `audit-wintf-graphics-world` | 付け替えの穴（P47）の本番は `src/ecs/graphics/systems/visual_sync.rs` |
| 84 | `ecs/test_support.rs` の持ち主（wintf の ECS の見直しのどれか） | 試験の窓の補助の置き場所 |
| 94・101 | `audit-wintf-layout-widget` | `FlexContainer`／`FlexItem`・`Opacity` の型 |
| 14・19 | 根の `Cargo.toml`・`.kiro/steering/tech.md` | `Win32_UI_Animation` の機能・`nonmax` の宣言・依存の登記 |
| 65 | `.kiro/steering/logging.md` | 118 行の手順（`doc/`・`docs/` を名指す行ではない） |

- 相手の見直しの spec は、どれもまだ要件の前（`spec.json` が無い）。要件 6.2 の申し合わせが要るのは、直すときに相手が走っていた場合。

## 6. 公開の口への響き（要件 4.4）

直すと crates.io の `wintf 0.1.0` から消えるか形が変わるもの: 1・12・13・14・15・16・17・18・20・24・66（keywords）・69・74・5（`Percent` の振る舞い）・101（`Opacity` は範囲の外）。どれも 0.x の壊す変更なので、次の版（v0.2.0 の見込み）の知らせに書く。`numerics` は消さないので響かない。

## 7. 直さずに残す候補（要件 3.2）

| 種類 | 番号・場所 | 中身 |
|---|---|---|
| ⑵ わざと置いた先送りの印 | 21・22・23・88 | 描画の木の部品（brief の決まり）。申し送り |
| ⑵ | 119 | 試し掘りの寿命の間の漏れ（61 番で消えるなら不要） |
| ⑴ 開発者の裁定 | 5 | 振る舞いを変えない「固定」として置かれたテスト（追跡の P49 は実在しない）。直すと公開の振る舞いが変わる |
| 指摘にしなかった判断 | `runtime/mod.rs:L150-L151`（CoUninitialize を呼ばない・P30）・`test_crash_report.rs`（プロセスの寿命の MTA＝候補 B・kernel32 の手書きの宣言・`shortcut:` L342）・`api_owner_chain_*_probe_tests.rs`（完了 spec の調べの檻）・`tick_bridge.rs:L401` の `#[ignore]`（`zorder-chain-residue` が持つ）・`hierarchy_reparent_gap_test.rs`（P47 の穴の固定）・`window_pos_systems_test.rs:L85-L166`（P40）・`surface_optimization_test.rs:L267-L290`（+inf の飽和）・`brushes_system_test.rs:L180-L203`・`examples/tooltip_demo.rs:L163-L164`・`taffy_flex_demo/main.rs:L318-L320` | 理由と限界を書いた判断。指摘に数えていない。P47 の穴（35 番）と P49（5 番）は持ち主の spec が無いので、要件の討議で扱いを確かめる |
| ⑶ SSP／ukadoc の互換 | なし | wintf の範囲に ukadoc の正典に関わる振る舞いは無かった |

## 8. 確かめた指摘

読み手の報告のうち、次は根拠の行を読み直して確かめた: 1・2・3・4・5（`dimension.rs` の文書と変換）・6（`create_windows` の問い合わせ）・7・8・9・10・12・13・14・15・16・17・18・19・20・25（bevy の最初の回の扱い）・26・27（`wintf` の語 0 件）・33・34・35（直し: 読み手は「同期済みの子」と書いたが、実際は移す前に同期していない）・36・37・38・39（試験の口が `#[doc(hidden)] pub`）・40・44・48・49・51・53・60・61・64・65・66・68・69（13 のメソッドの `git grep -w`）・71。残り（主にテストの重なりと同語反復、注記の食い違い）は読み手の報告の行の番号と根拠をそのまま載せた。設計の段で直すときに、もう一度その行を読む。

---

## 9. 要件と今の姿のつき合わせ（ギャップ分析）

| 要件 | 今の姿 | すき間 |
|---|---|---|
| 1 範囲の確定 | 160 ファイルを数えて一覧にした（1 節） | なし |
| 2 洗い出し | 160/160 を読み、119 件を 3 つの組に分けた（3 節） | なし |
| 3 対象の確定 | 残す候補・申し送り・重なりを印にした（4・5・7 節） | 要件の討議で裁定が要る（下の議題） |
| 4 全部直す | まだ直していない | 119 件。公開の口の一覧（6 節）を次の版の知らせへ |
| 5 守る決まり | 1,000 行を越えるファイルは範囲に無い（最大 971 行の `api_owner_chain_probe_tests.rs`） | 直しで足すテストが大きいファイル（`runtime/mod.rs` 833 行・`win_style.rs` 861 行）に入りきらないときは兄弟のファイルへ |
| 6 範囲の外 | 5 節の表 | 相手の spec がまだ始まっていない |
| 7 壊れたら落ちるテスト | 必ず直すの 5 件のうち、1 は消して解く・2 は相手の spec のテスト・3・4・5 は新しいテストが要る | 3・4 のテストを足す。7・8・9・10 のログの直しは log-capture-kit で確かめられる |
| 8 かけ直し | 未 | 直した後に同じ作法で |

### 直し方の選び方

- **A 案（ここで全部・1 本）**: 119 件を 1 本の spec で直す。範囲が 1 つで申し合わせが少ない。見立ては下のとおり 20 タスクを越える。
- **B 案（範囲を 2 本に割る・推す）**: 1 本目＝本番のコード（`src/`・`Cargo.toml`・`build.rs`）と example（必ず直す 1〜4・直したほうがよい 6〜20・60〜68・できれば 69〜88・112・117〜119）と、本番の削除と同時に消さないとコンパイルが通らないテスト（24・28・29 と、14・16・17・18・69 が消す口を呼ぶテスト）。2 本目＝残りのテストのファイル（`tests/`。5・25〜27・30〜59・89〜111・113〜116）。1 本目が先に着地し、2 本目はその後。21〜23 は申し送りで、どちらでも直さない。
- **C 案（組で割る）**: 1 本目＝必ず直す＋直したほうがよい（68 件）、2 本目＝できれば（51 件）。brief の書き方に合うが、同じファイルを 2 本が別々に触るので、1 本目の削除で 2 本目の指摘が古びやすい。

### 手間と危うさ

- **手間: L（1〜2 週）**。削除が多く、それぞれは小さいが、119 件・40 を越えるファイルに及ぶ。新しい形は無い。
- **危うさ: 中**。公開の口を消す（0.x の壊す変更）・テストを直すと今まで隠れていた本番の後退が赤として出うる（25・34・49・50・53 は直した瞬間に今のバグが見える見込み）・GPU のテストの持ち主のスレッドの寄せ（45）は実 GPU で確かめる。

### タスクの数の見立て（要件 3.7）

| まとまり | タスク |
|---|---:|
| com の削除（13・14・16・24・69・70）と dwrite の失敗の直し（4）・wic（15・28・29） | 4 |
| runtime の配置とログ（3・6〜11）と注記・死んだコード（71〜81） | 4 |
| win_state・win_style・依存・ambassador（17〜20・85・86）と api・調べの注記（82〜84・87） | 3 |
| DWMFLUSH（2・8）＝相手の spec へ渡すなら 0 | 0〜1 |
| tests/com・drag・ecs・win_app（24〜33・90〜92） | 3 |
| tests/graphics（34・37〜39・44・48・104〜107・113・114） | 3 |
| tests/visual・widget・window（35・36・40〜43・45〜47・108〜111・115・116） | 3 |
| tests/layout（5・49〜59・93〜103） | 3 |
| examples と資産（60〜68・89・112・117〜119） | 2 |
| 公開の口の一覧・tech.md／logging.md の直し | 1 |
| かけ直しの audit と全体テスト | 1 |
| 計 | **27〜28** |

- **20 タスクを越える見込み**。要件 3.7 に従い、要件の討議で 2 本目に切る案を出す（上の B 案なら 1 本目 約 14・2 本目 約 13）。

### 設計へ持ち越す調べ（Research Needed）

- 15 番: `IWICPixelFormatInfo2::SupportsTransparency` で手書きの一覧を置き換えるか、`has_alpha` ごと消すか（areka-emo-atlas の写しをどうするかと組）。
- 25・34・49・50・53 番: 確かめを直したとき、今の本番が緑のままか（赤になれば、その本番の直しは持ち主の見直しの spec へ）。
- 45 番: 見た目の実行ファイルを持ち主のスレッドへ寄せたとき、実 GPU で全部緑か。
- 30 番: 単一スレッドのスケジュールを決定論で固定する最小の本番の口（本番は `audit-wintf-graphics-world` が直す・こちらはテストを消す）。
- 84 番: 試験の窓の補助を範囲の中（`src/api.rs` の `#[cfg(test)]`）に置くか、範囲の外の `ecs/test_support.rs` に置くか（2026-11-05 の討議で設計へ回した）。
- 2・8 番: `wintf-dwmflush-wait-compensation` の brief の形（呼ぶ前後の時間で判定・不足分だけ眠る・ループの最後で必ず 1 回起こす・注記を直す）を、この spec の設計で写す（討議で吸収と決めた）。

## 10. 要件の討議へ渡す議題（答えで作業が変わるもの）

1. **`DwmFlush` の重なり（2・8 番）をどちらで直すか**: `wintf-dwmflush-wait-compensation`（brief が形を決め済み・規模 S）で直し、この spec は記録と注記（`mod.rs:L337`・`L365`）だけにする案を推す。
2. **描画の木の申し送り（21・22・23・88 番）**: 消さずに ⑵ として `wintf-visual-tree` へ渡す（brief の決まりどおり）。`moved` の単位の誤りもあちらで直す。
3. **公開の口を消す直し（6 節の 15 件）**: 0.x の壊す変更として次の版でまとめて消す案を推す（先に `#[deprecated]` を 1 版挟む案もある）。
4. **範囲の外の本番の直し**: 5 番（百分率の 100 倍・公開の振る舞いが変わる）・35 番（付け替えの穴 P47）・30 番の定数・94／101 番の死んだ型は、持ち主の見直しの spec（`audit-wintf-layout-widget`・`audit-wintf-graphics-world`）で直し、この spec はテストの側だけを直す案を推す。5 番は ⑴（「固定」として置かれた）に当たるかも確かめる。
5. **知っている穴を固定するテスト**（7 節の「指摘にしなかった判断」の P47・P40・+inf の飽和ほか）: 判断として残す。持ち主の spec の無い P47・P49 の扱いを決める。
6. **20 タスクを越える**: B 案（本番と example／テスト）で 2 本に切る案を推す。
7. **依存を減らす（19・20 番）**: ウェーブの約束は「足さない」だけなので減らすのは進めてよいか。根の `Cargo.toml`（`nonmax` の宣言・`Win32_UI_Animation` の機能）と `tech.md` の行も直す。
8. **example を消す（60〜63・65・89・112）**: 公開の荷の example が減る。`logging.md:118` の手順の向け直し（65 番）を含めてよいか。
9. **試験の窓の補助の置き場所（84 番）**: 範囲の外の `ecs/test_support.rs` に置くか、範囲の中（`src/api.rs` の `#[cfg(test)]`）に置くか。
10. **テストが本物の窓を画面に出す（54・81 番）**: 見えない窓にする・確かめる前に隠す、を進めてよいか（実機の入力テストと同じ机で走る）。
11. **見た目のテストを GPU の持ち主のスレッドへ寄せる（45 番）**: 2 つの根本原因の説明のどちらを正とするか。実 GPU で確かめてから決める。

### 討議の結果（2026-11-05）

- 9・11 は設計へ回した（上の「設計へ持ち越す調べ」）。
- 1 は推奨と違い、この spec に吸収と確定した（触る場所が同じ小さな直しは 1 本にまとめる）。
- 4 は推奨と違い、5・94・101 番の本番もこの spec で直すと確定した（持ち主の `audit-wintf-layout-widget` は次のウェーブで並走が無い）。30・35 番の本番は `audit-wintf-graphics-world`。
- 6 は推奨（2 本に切る）を採らず、開発者「1 本のまま」で確定した。
- ほかは推奨案で確定した。確定の正本は `requirements.md` の「指摘の扱い」。

---

## 11. 設計の段の調べ（2026-11-05・`/kiro-spec-design -y`）

### 要約

- **Feature**: `audit-wintf-runtime`
- **調べの型**: 既存の仕組みの手直し（Extension）＝軽い調べ。新しい依存は無い。外の資料は、`windows` 0.62.2 の型の在り処を手元の registry の源（`windows-0.62.2/src/Windows/Win32/Graphics/Imaging/mod.rs`）で確かめただけ。
- **使ったスキル**: `ponytail`（いちばん小さな完全な変更・消す前に grep・呼び出し元まで直す）。直し方は全部この物差しで選んだ。
- **主な見つけ**:
  1. 15 番の「`has_alpha` ごと消す」案と 29 番の「見本を写す」案は、完了した `emo-atlas-palette-alpha` の要件 3（wintf の読み込みも読む前の α の有無を返し、emo-atlas と同じ答え）と、同じ spec の設計の討議の議題 1（見本は 1 か所・wintf は相対パスで読む・2026-10-31 開発者の一括承認）を覆す。要件 4.5 に従い、どちらも覆さない形を選んだ（下の「設計の決め」）。
  2. 33 番の「MTA か」の確かめは、テストのプロセスがプロセスの寿命の MTA を持つと、初期化していないスレッドでも `APTTYPE_MTA`（暗黙の MTA）と答えるので、それだけでは壊しても緑のまま。限定子が `APTTYPEQUALIFIER_IMPLICIT_MTA` でないことまで見る。
  3. 30・35 番の本番と、`audit-wintf-graphics-world` が書き換えてよいテストのファイル 12 本に触る指摘は、あちらの着地の後に回す。こちらの削除があちらのファイルの注記に古い名前を残すもの（72・73・101・61）も、あちらの着地の後にまとめて合わせる。

### 調べの記録

#### 消す前の参照の確かめ（要件 4.3）

`git grep -n -w <名前> -- . ':!vendors'`（`.kiro/` の文書は除いて数えた・基点は main `88eb0a43` と同じ木）。「定義の外」は、定義のファイルと、いっしょに消すテストのファイルを除いた数。

| 番号 | 消すもの | 定義の外の参照 | 補足 |
|---|---|---:|---|
| 1・13・24 | `RecCommandSink`・`command_sink`・`command_types`・`dup_com` | 0 | 言及は `tests/com/d2d_ext_test.rs` の 6-7・117・216-219・325-367 行だけ（いっしょに消す・直す） |
| 12 | `drain_dispatcher_queue`・`drain_step`・`pump_current_thread_messages` | 0 | `wuc.rs:65`・`:105` の `wuc_spike` への言及も同じ所で消える |
| 14 | `com::animation`・根の `Win32_UI_Animation` | 0 | `UI::Animation`・`UIAnimation` は `animation.rs` と `tests/com/animation_test.rs` だけ。`.kiro/steering/structure.md:37` の行も直す |
| 16 | `D2D1CommandListExt::open` | 0 | `close` は `areka-emo-present/src/display.rs:31` ほかが使うので残す |
| 17 | `WinState`・`SimpleWinState`・`win_state` | 0 | 言及は `lib.rs:8`・`:14` と `.kiro/steering/structure.md:121` だけ（どちらも直す） |
| 18 | `WinStyle`・`set_style`・`set_ex` | 0 | `set_style` の 1 件（`ecs/layout/systems/taffy_systems.rs:77`）は taffy の別の関数。`apply_click_through`・`apply_layered_companion` は `ecs/clickthrough/controller.rs:28` が使うので残す |
| 19 | `nonmax` | 0 | `Cargo.toml` 2 か所と `THIRD-PARTY-NOTICES.md` だけ。謝辞は完了の全体テスト（`-License`）が作り直す |
| 19 | wintf の開発用の `rand` | 0 | 根の宣言は `dola` が使うので残す |
| 19 | `bevy_app`（本番から開発用へ） | `src/` で 0 | `tests/graphics/surface_optimization_test.rs:24`・`:26`・`:68` だけ。根の宣言は残す |
| 20 | `ambassador`・`#[delegatable_trait]` | 0 | `#[delegate(` は 0。印は `win_state.rs:8`・`d2d/mod.rs:94` の 2 か所だけ。根の `[workspace.dependencies]` の行も消す（ほかに使う crate が無い） |
| 69 | `D2D1FactoryExt`（trait ごと）・`create_bitmap_from_wic_bitmap`・`draw_text`・`fill_geometry`・`draw_image`・`D3D11DeviceExt`（trait ごと）・`DWriteTextFormatExt`（trait ごと）・`get_cluster_count`・`hit_test_text_position` | 0 | `draw_image` の 1 件は `ecs/widget/bitmap_source/systems.rs:186` のコメント |
| 69 | `D2D1DeviceExt::create_command_list` | **4** | `tests/graphics/command_list_test.rs:22`・`:35`・`:45`・`tests/graphics/surface_systems_test.rs:266`（設計の検証で見つけた 4 か所目）。ギャップ分析の「`tests/com/` の外で 0」は誤り。そのテストは装置の文脈から `CreateCommandList()` を呼ぶ形へ直す |
| 71 | `MessageLoopDriver::run`・`default_filter` | 0 | 同じファイルのテスト（`:203-213`）だけ |
| 73 | `MessageLoopDriver`・`AsyncTickTask` | 0（本番の呼び手は `runtime/` の中だけ） | 名前の言及が `ecs/world/mod.rs:143`・`:784`・`ecs/world/vsync.rs:18`・`:60`・`:74`（`audit-wintf-graphics-world` の持ち物）と `message_loop.rs:166` に在る。あちらの着地の後に言及ごと直す |
| 72 | `notify_shutdown`（`request_exit` へ畳む） | 0 | 言及が `ecs/app.rs:51`（あちらの持ち物）に在る。同上 |
| 74 | `WinApp::spawn_ui_local` | 1 | `examples/dcomp_demo.rs:76` だけ（112 番でファイルごと消す） |
| 60・61・62・65・89・112 | example と資産 | 0 | `dcomp_demo.jpg`・`demo_checker_64x64.png`・`seikatu_0_1`／`0_2`／`1_0`／`1_1`／`1_2.webp` は 0。`seikatu.webp` は `split_image.rs:7` だけ。`multi_window_test` は `.kiro/steering/logging.md:118` だけ。`wuc_spike` は `com/wuc.rs:65`・`:105`（12 番で消える）と `ecs/world/mod.rs:152`（あちらの持ち物の注記）。`seikatu_0_0.webp` と `demo_region_colormap_64x64.png` は example が読むので残す |
| 94 | `FlexContainer`・`FlexItem` | 0 | 型としての利用は 0（ほかの一致は example の文字列と `Name`） |
| 101 | `Opacity`（`ecs/layout/metrics.rs`） | 0 | 型としての利用は 0。`ecs/graphics/visual.rs:140` の注記が名前を引く（あちらの持ち物）。あちらの着地の後に注記を直す |

**消す直前の確かめ（タスク 1.1・C1・2026-11-05）**: `git grep -n -w -e RecCommandSink -e command_sink -e command_types -e dup_com -e D2D1FactoryExt -e create_path_geometry -e create_rounded_rectangle_geometry -e create_bitmap_from_wic_bitmap -e draw_text -e fill_geometry -e draw_image -e delegatable_trait -- . ':!vendors' ':!.kiro'` を消す前に引き、定義のファイルと `tests/com/` の外の参照は 0 件（表の 1・13・24・16・69 のとおり）。消した後に同じ grep を引き直した残りは、範囲の外の 3 種だけ: `src/ecs/widget/bitmap_source/systems.rs:186` の注記（`draw_image` を一般の語として引く）・`src/win_state.rs:8` の `#[delegatable_trait]`（17・20 で W1 が消す）・`doc/archive/spec/` の古い擬似コードの `draw_text`／`draw_image`（wintf の口ではない）。`D2D1CommandListExt::open` の呼び手も 0 件（`.open()` の grep で `crates/wintf` に 0）。`create_command_list` は 7.3（T2b）まで残す。

**消す直前の確かめ（タスク 1.2・C2・2026-11-05）**: `git grep -n -w -e D3D11DeviceExt -e DWriteTextFormatExt -e get_cluster_count -e hit_test_text_position -e get_device_removed_reason -e create_texture2d -e set_text_alignment -e set_paragraph_alignment -- . ':!vendors' ':!.kiro'` を消す前に引き、定義のファイル（`src/com/d3d11.rs`・`src/com/dwrite.rs`）と、いっしょに消すテスト（`tests/com/d3d11_test.rs`・`tests/com/dwrite_test.rs`）の外の参照は 0 件（表の 69 のとおり）。`HitTestResult` の同名は `ecs/layout/hit_test/mod.rs` の別の型で、dwrite の型だけを消した。消した後に同じ grep（`-e HitTestTextPosition` を足して）を引き直して 0 件。91（d3d11）は、通しのテスト `create_device_fills_optional_out_params` が繰り返す煙のテスト `create_device_without_out_params_succeeds` を消した（research の行番号 L216-L219 は今の木では古い）。92（dwrite）の `hit_test_text_position_clamps_out_of_range_position` は口ごと消えた。4 番の較正: `sizing_outcome` を戻り値を捨てる形（常に `Ok(true)`）にすると `com::dwrite::tests::sizing_outcome_ok_means_no_clusters` と `sizing_outcome_other_failure_is_err_with_one_error_log` が赤になることを確かめてから戻した。

**消す直前の確かめ（タスク 1.3・C3・2026-11-05）**: `git grep -n -w -e drain_dispatcher_queue -e DrainStep -e drain_step -e pump_current_thread_messages -e UIAnimation -e animation_test -- . ':!vendors' ':!.kiro'` と `git grep -n -e com::animation -e UI::Animation -e com/animation -e com/dcomp -- . ':!vendors' ':!.kiro'` を消す前に引き、定義のファイル（`src/com/wuc.rs`・`src/com/animation.rs`・`src/com/mod.rs:1`）と、いっしょに消すテスト（`tests/com/animation_test.rs`・`tests/com.rs` の宣言）の外の参照は 0 件（表の 12・14 のとおり。他の crate の `mod animation_tests` は別物）。`tests/com/common` の `with_com_initialized` は `d2d_ext_test`・`wic_test` も使うので残した。消した後に同じ grep を引き直した残りは `Cargo.toml:83` の `Win32_UI_Animation`（タスク 6 で消す）と `.kiro/steering/structure.md` の行（タスク 6 で直す）だけ。`wuc_spike` の言及は `wuc.rs` から消え、残りは `examples/wuc_spike.rs` 本体と `ecs/world/mod.rs:152`（表の 60 ほかの行のとおり）。70 は `wuc.rs` の `com/dcomp.rs` と移行前の DComp 版を引く文 3 か所を消し、`begin_draw` の注記は何を返すかだけにした。

**15 番の実測と較正（タスク 1.4・C4・2026-11-05）**: 手書きの 12 の形式（`32bppBGRA`・`32bppPBGRA`・`32bppRGBA`・`32bppPRGBA`・`64bppRGBA`・`64bppPRGBA`・`64bppBGRA`・`64bppPBGRA`・`32bppRGBA1010102`・`32bppRGBA1010102XR`・`128bppRGBAFloat`・`128bppPRGBAFloat`）は、この机の WIC（Windows 11 10.0.26300）で `CreateComponentInfo` → `IWICPixelFormatInfo2::SupportsTransparency` が 12 とも TRUE を返した（FALSE の形式は無いので、名前つきの定数は残さない）。`16bppBGRA5551`・`64bppRGBAHalf`・`40bppCMYKAlpha` も TRUE、`24bppBGR`・`32bppBGR`・`8bppGray`・`48bppRGB` は FALSE、形式の情報が無い GUID（全ゼロ）は WIC のエラー。消したのは定数の一覧だけで、名前の参照は両方の crate の中に閉じる（`ALPHA_FORMATS` の `git grep` は消した後 0 件）。較正: 判定を手書きの 12 の一覧へ戻した形で、`com::wic::tests::pixel_format_has_alpha_asks_wic`（wintf）と `decode::wic_arm::animation_tests::pixel_format_has_alpha_asks_wic`（areka-emo-atlas）が「16bppBGRA5551」で赤になることを確かめてから戻した。パレットの 6 本の見本の答え（`tests/com/wic_test.rs` の `palette_*`・`wic_arm_palette_tests.rs`）は緑のまま。

**28・29・90・91（wic）・116（bitmap_source）の直しと較正（タスク 1.5・C4・2026-11-05）**: 28 は全体の写しを補助 `copy_whole_pbgra` に寄せて「0 でない画素がある」を確かめ、部分の写し（`copy_pixels_with_subrect_matches_whole_copy`・原点からずらした (1,1) の 2x2）を全体の写しの同じ画素と比べる。較正: ⑴ `WICBitmapSourceExt::copy_pixels` を何も写さず `Ok(())` を返す形に壊すと `wic_test::copy_pixels_whole_image_fills_buffer` と `wic_test::copy_pixels_with_subrect_matches_whole_copy`（と前からの `copy_pixels_with_undersized_buffer_fails`）が赤、⑵ 部分の矩形の X・Y を 0 に置き換える形に壊すと `copy_pixels_with_subrect_matches_whole_copy` だけが赤（中身の比べで落ちる）。どちらも戻した。29 は `build.rs` が `../areka-emo-atlas/src/testdata/palette` の在る無しを見て `cfg(areka_palette_samples)` を出し（`cargo::rustc-check-cfg` で宣言・MSVC でないときの早い戻りより前）、`palette_*` の 6 本に `cfg_attr(not(areka_palette_samples), ignore = ...)` を付けた（見本は写さない）。ワークスペースの中で `cargo test -p wintf --test com wic_test` は 13 本 ok・0 ignored。較正: cfg を出す行を消すと `wic_test::palette` の 6 本が「6 ignored」になることを確かめて戻した。90 は `tests/widget.rs` に `#[path = "com/common/mod.rs"] mod common;` を足し、`bitmap_source_integration_test.rs` の写しの 2 つの補助と `tests/com/common/mod.rs` の 2 つの `#[allow(dead_code)]` を消した。91（wic）は他の全部のテストが通る道を繰り返す `wic_factory_can_be_created` を、116（bitmap_source）は `!is_null()` だけの `test_wic_core_creation_in_integration`・`test_wic_core_clone_in_integration` を消した。消す前と後の `git grep -n -e wic_factory_can_be_created -e test_wic_core_creation_in_integration -e test_wic_core_clone_in_integration -e copy_pixels_with_subrect_succeeds -- . ':!vendors'` は、定義の外では `.kiro/specs/completed/` の昔の走行の記録と一覧（読む道具は無い・`git grep "golden/wintf"` は `.kiro` の外で 0 件）だけ。

**2・8・11・77・78 の直しと較正（タスク 2.1・R1・2026-11-05）**: `vsync_loop` は `while !stop { vsync_once(..) }` になり、1 周（呼ぶ前の時刻 → `DwmFlush` → `Err` なら `debug!` に HRESULT → 呼ぶ前から 1ms 未満なら 16ms に足りない分を眠り `trace!` → 必ず 1 回 notify）を `vsync_once`、補いの長さを `compensation`、待ち合わせを `join_vsync`（panic なら `error!`）、`run` の中の中継を `relay_vblank`（`tick_bridge.rs`・`run` は `spawn_local` するだけ）へ出した。`stop_flag → join` の順は変えていない。注記は module doc の「後続タスク」「本タスクの時点では結線は行わず」、`AsyncTickTask` の「固定 60Hz のフォールバック」、`run` の「60Hz」の 2 か所、`run_async_tick`・中継の「自分で終わる」（正常な終了の後は起床が来ず待ちのまま残る・`run` 1 回につき 1 度の小さな残り）を直した。テストは `src/runtime/tick_bridge_vsync_tests.rs`（6 本）。較正（どれも本番へ 1 度入れて赤を見て戻した）: ⑴ `vsync_once` の `Err` の回で notify の前に `return` すると `vsync_once_failed_flush_logs_sleeps_and_still_notifies` が赤。⑵ `compensation` の線を `waited <= WAITED_AT_LEAST` に変えると `compensation_boundaries` が赤（1ms が補われる）。⑶ `join_vsync` の `error!` を `trace!` に戻すと `join_vsync_leaves_one_error_for_a_panicked_thread` が赤。⑷ `relay_vblank` の `!running.get()` の判定を消すと `relay_stops_without_waking_once_the_loop_has_returned` が赤。旗を切った形（旧の `Ok` だけ notify・`Err` は 15ms 眠るだけ・`trace!`・判定なし）では 6 本中 4 本が赤（`compensation_boundaries` と待てた回の 1 本は新しい口そのものの検査なので緑）。

**3・6・9・10・75・77・80・81 の直しと較正（タスク 2.2・R2・2026-11-05）**: 生成直後の位置合わせの旗を `initial_pos_flags(x, width)`（両方既定なら `None`・位置が既定なら `SWP_NOMOVE`・大きさが既定なら `SWP_NOSIZE`）で決め、座標の換算（`to_window_coords_for_creation` の素通し）は変えていない。`new_ex` の失敗は `mark_creation_failed`（`error!` 1 行＋公開の印 `WindowCreationFailed`・`wintf::WindowCreationFailed` として再公開）へ出し、`create_windows` の問い合わせに `Without<WindowCreationFailed>` を足した（`ecs/window/window_system.rs`・範囲の外の 1 行の追従。持ち主の `audit-wintf-window` のセッションは無いと進行役が確かめた）。`SetParent`・2 つの `SetWindowPos`・`SetWindowTextW` は `if let Err(e)` で `error!` を残して先へ進み、登録簿をその場で作る枝は `debug!` を `error!` にした。doc は 75（`new_checked_ex` → `new_ex`・World を借りたまま作り生成時のメッセージは借用中の道で飛ばす）・77（`window_factory.rs` の「後続タスク 4.3」・`window_registry.rs` の「本タスクの領分外」「後続タスク 4.2」の 3 か所）を実際に合わせた。80 は 2 つの import と `let _ = (..)`・`let _ = registry;`（3 か所）を消した。81 は 2 本のテストの doc の「不可視」を直し、作った直後に `ShowWindow(SW_HIDE)` で隠してから確かめる形にした。テストは `src/runtime/window_factory_creation_tests.rs`（6 本）。較正（旗を切った形＝設計のテストの方針の壊し方をそのまま入れた形で 1 度回し、6 本とも赤を見てから旗を外した）: ⑴ 3＝片方だけのとき呼ばない旧の形で `initial_pos_flags_four_cases`（`None` と `Some(..)` の食い違い）と `position_only_window_is_created_at_the_given_position`（左上が (26,26)・期待 (123,77)）が赤。⑵ 6＝印を入れないと `creation_failure_marks_entity_and_logs_one_error` が赤、問い合わせの除外を外すと `create_windows_skips_marked_entity_until_the_mark_is_removed` が赤（どちらのテストも片方の壊し方だけに依る）。⑶ 9＝`SetParent` を記録しない形で `broken_parent_logs_error_and_still_creates_the_window` が赤（`error` 0）。⑷ 10＝`debug!` のままで `missing_registry_logs_one_error` が赤（`error` 0・期待 1）。

**7・71・74・75・76・77・79・81・112 の直しと較正（タスク 2.3・R3・2026-11-05）**: `with_exit_policy` の COM の初期化を `init_ui_com`（`runtime/mod.rs`・非公開）へ出し、`RPC_E_CHANGED_MODE` のとき `warn!` を 1 行残して `Ok`、`S_FALSE` は黙って `Ok`、ほかの失敗は `Err`。テストは `src/runtime/ui_com_init_tests.rs`（2 本・どちらも新しいスレッドで回す）。較正: `warn!` を出さない形（旗を切った形＝`warn!` を消した形と同じ）で `already_sta_thread_is_ok_with_one_warn` が赤（warn 0・期待 1）、`uninitialized_thread_is_ok_without_warn` は緑を見てから旗を外した。消す前の `git grep -n -w -e default_filter -e spawn_ui_local -e dcomp_demo -e default_filter_forwards_arbitrary_message -e shutdown_future_completes_when_event_notified -e notify_shutdown_wakes_armed_listener_and_is_idempotent -- . ':!vendors' ':!.kiro'` と `git grep -n "MessageLoopDriver::run"` は、定義のファイル（`runtime/message_loop.rs`・`runtime/mod.rs`）と、いっしょに消す `examples/dcomp_demo.rs` の外で 0 件（表の 71・74 のとおり。ほかの crate の `MessageLoop::run` はライブラリの口で別物）。消した後に同じ grep を `.kiro/specs/` だけを除いて（`.kiro/steering/` も含めて）引き直した。`.kiro/steering/structure.md:113` の `WinApp` の口の並びにあった `spawn_ui_local` は消した。残りは `.kiro/specs/` の記録と、`src/ecs/world/mod.rs:151` の注記（昔の観測の場として `dcomp_demo` の名を引く・`audit-wintf-graphics-world` の持ち物・コードの参照ではない）だけ。この注記はタスク 7.8（G5「こちらの削除で古くなった名前の言及をツリー全体で 0 件にする」）で直す。`Cargo.toml` に `[[example]]` の宣言は無い。`examples/dcomp_demo.jpg` は X1（60 番）が消す。doc は 75（mod.rs の `new_checked_ex` → `new_ex`・World を借りたまま作り生成時のメッセージは借用中の道で飛ばす）・76（`wire_new_path` の「冪等」「複数回呼んでも安全」を消し、1 度呼ぶ・冪等ではないと書いた）・77（`WinApp` の「結線は後続タスクで追加する」・`message_loop.rs` の module doc の「後続タスクで結線する」・`block_on` の「後続タスクで利用する」）を直した。79 は `event_listener` だけを試す 2 本を消した。81 は `close_to_reconcile_to_shutdown_chain_wakes_listener` の doc の「不可視」を直し、窓を作る 2 本（と `explicit_policy_keeps_the_loop_alive_when_the_last_window_closes`）で作った直後に `SW_HIDE` で隠すようにした。

**17・18・20・84 の直しと較正（タスク 3.1・W1・2026-11-05）**: 消す前に `git grep -n -w -e WinState -e SimpleWinState -e win_state -e WinStyle -e set_style -e set_ex -e delegatable_trait -- . ':!vendors' ':!.kiro/specs'` を引き、定義のファイル（`src/win_state.rs`・`src/win_style.rs`）と `src/lib.rs` の宣言・再公開の外は、表の 17・18 のとおり `.kiro/steering/structure.md:121` の `win_state.rs` の行と `ecs/layout/systems/taffy_systems.rs:77` の taffy の別の `set_style` だけ。`win_state.rs`（`#[delegatable_trait]` と `use ambassador::*` ごと・テスト 6 本）、`lib.rs` の `mod win_state;`・`pub use win_state::*;`、`win_style.rs` の `#![allow(non_snake_case)]`・`WinStyle`（組み立て役と `commit`／`new` ほか）・`set_style`・`set_ex`・そのテスト 23 本を消した。`apply_click_through`・`apply_layered_companion` とそのテスト 4 本は残した。消した後に同じ grep を引き直した残りは `structure.md:121`（タスク 6 で直す）と taffy の `set_style` だけ。`delegatable_trait` は 0 件、`ambassador` の残りは根と wintf の `Cargo.toml`・`Cargo.lock`・`THIRD-PARTY-NOTICES.md`・`.kiro/steering/tech.md:35`（どれもタスク 6 の持ち物）。84 は `api.rs` の `#[cfg(test)]` に `create_test_window(ex, style, parent, title)`（`api.rs` のテストの中に在ったものを外へ出し拡張の形の引数を足した）と `with_test_window(ex, style, parent, f)`（作る・`catch_unwind` で本体を回す・必ず `DestroyWindow`・panic を投げ直す）を置き、`api.rs` のテスト・`win_style.rs` の 2 本・`win_style_accept_files_tests.rs`・`api_owner_chain_probe_tests.rs` の `create_probe_window`（補助で作ってから `SW_SHOWNOACTIVATE` で見せる・位置は (10,10) から (0,0) へ）で使う。`ecs/` の下は触っていない。調べの 11 本（と `api_owner_chain_nudge_probe_tests` の 2 本）は (0,0) でも緑のまま。補助のテスト `api::tests::with_test_window_destroys_the_window_and_rethrows_on_panic` を足した。較正: `with_test_window` の `DestroyWindow` を外す形に壊すとこのテストが赤（窓が生きている）になることを確かめてから戻した。

**25・26・32・33・91（drag）・92（tree）の直しと較正（タスク 4.1・T1・2026-11-05）**: 26 は消す前に `git grep -n capture_guard_panic_safety -- . ':!vendors' ':!.kiro/specs'` を引き、`tests/drag.rs` の宣言 2 行の外は `src/ecs/drag/state/tests.rs:11` の注記の言及 1 件だけ（型や関数の参照は 0・注記は `src/ecs/` の持ち主の範囲なので残す）。ファイルと宣言を消した後は注記の 1 件だけ。25 は `tree_propagation_test.rs` の 2 本を、同じ `mark_dirty_trees` の system を 1 度回して前回の刻みを据え、`world.read_change_tick()` を控え、`clear_trackers()` で刻みを 1 つ進めてから葉（または `ChildOf`）を変えてもう一度回し、根の `ArrangementTreeChanged` の `get_change_ticks().is_changed(控え, 今)` を見る形にした（確かめの system と `DirtyProbe` は消した）。控えを `clear_trackers()` の後に取ると、葉の書き込みと system の刻みが控えと同じ値になって正しい本番でも赤になる（bevy の `increment_change_tick` は前の値を返す）ので、控えは進める前に取る。92（tree）は `propagate_is_idempotent_when_nothing_changes` の doc を「冪等（飛ばしたかどうかは見分けない）」に直した。32 は `window_pos_sync_updates_size_only_for_dragging_entity` を消し、`window_pos_sync_resumes_after_dragging_removed` の最初の大きさを 640x480 にした。そのとき、生成の束に入れた `GlobalArrangement` は `Arrangement` の `on_add` フック（`src/ecs/layout/arrangement.rs:39-47`）が既定値で上書きするので、元のテストの 1 回目は大きさ 0 で読み飛ばされていた（大きさを同じ値にしていたので見えなかった）と分かった。テストは生成の後に 1 度回してから境界を入れる形にした（本番は直さない）。91（drag）は `mixed_entities_only_non_dragging_updated` が繰り返す `sync_arrangement_skips_dragging_entity`・`sync_arrangement_processes_non_dragging_entity` を消した。33 は `win_app.rs` に `CoGetApartmentType` が `APTTYPE_MTA` かつ限定子が `APTTYPEQUALIFIER_IMPLICIT_MTA` でないこと、`AreDpiAwarenessContextsEqual(GetThreadDpiAwarenessContext(), PER_MONITOR_AWARE_V2)` を足した。較正（本番に 1 度入れて赤を見てから戻した）: 25＝`mark_dirty_trees` のループを `.take(0)` で空にすると `mark_dirty_propagates_change_from_leaf_to_root`・`mark_dirty_reacts_to_childof_change` が赤（ほかに `propagate_single_branch_change_preserves_sibling`・`propagate_recomputes_child_after_reparent` も赤）。32＝ドラッグ中の `window_pos.size = Some(new_size)` を消すと `window_pos_sync_resumes_after_dragging_removed`・`window_pos_sync_propagates_size_change_during_drag` が赤。33＝`with_exit_policy` の `init_ui_com()?` を消すと `new_initializes_com_dpi_and_returns_usable_world` が「暗黙の MTA」（限定子 1）で赤、`ensure_process_dpi_awareness()` を消すと同じテストが DPI の確かめで赤。戻した後は `cargo test -p wintf --test win_app --test drag --test ecs` が 2・14・44 本とも緑。直したテストは本番のままで緑＝25 の判定の規則の新しい指摘は無い。

**43・46・108・116（monitor_hierarchy）の直しと較正（タスク 4.2・T3・2026-11-05）**: 消す前と後に `git grep -n -w -e test_mouseleave_scoped_pointer_clear -e test_drag_hwnd_guard_owner_window_check -e test_existing_tests_still_pass -e test_monitor_hierarchy_construction -e test_monitor_to_taffy_style_conversion -e test_text_direction_enum -e test_label_has_direction -e test_text_layout_metrics -- . ':!vendors' ':!.kiro/specs'` を引き、消すテストの定義の外は 0 件（`tests/layout/metrics_test.rs` の `test_text_layout_metrics_default_is_zero` などは別の名）。43 は `test_taffy_tree_sync_and_layout_computation` の終わりを、各 Monitor の `TaffyComputedLayout` が `TaffyComputedLayout::default()` と違うことを確かめる形にし（中身は `pub(crate)` で読めないので既定値との比べ）、`test_existing_tests_still_pass` を消した（`test_backward_compatibility_without_layout_root` が覆う）。46 は `multiwindow_event_test.rs` の WM_MOUSELEAVE の写しとドラッグの守りの 2 本と module doc の 2 行・使わなくなった `PointerState` の import を消した。108 は `vertical_text_layout_test.rs` の 3 本と要らない `#[cfg(test)] mod tests` を消し、残る 2 本を素の関数にした。116（monitor_hierarchy）は `test_monitor_hierarchy_construction`・`test_monitor_to_taffy_style_conversion` を `test_monitor_enumeration` の各 Monitor のループへ畳み（`position == Some(Absolute)`・`size`・`inset`・`Arrangement`・`GlobalArrangement`・モニター 1 枚以上は前からの確かめ）、`println!` を消した。較正: `compute_taffy_layout_system`（`src/ecs/layout/systems/taffy_systems.rs`）の `*computed = new_layout;` を書かない形に壊すと `monitor_hierarchy_test::test_taffy_tree_sync_and_layout_computation` が赤（直す前の形は存在だけを見るので緑のまま）になることを確かめてから戻した（戻した後の内容は HEAD と同じ blob）。戻した後は `cargo test -p wintf --test window --test widget` が 19・13 本とも緑。本番のままで緑＝新しい指摘は無い。

**5・52・93・94・101・103 の直しと較正（タスク 4.3・L1・2026-11-05）**: 5 は `src/ecs/layout/dimension.rs` の `From<LengthPercentageAuto>`・`From<LengthPercentage>` の百分率の枝に `/ 100.0` を足し（`Dimension::Percent` と同じ）、`tests/layout/dimension_conversion_test.rs` の期待を `percent(0.5)`・`percent(0.25)` に裏返し（`test_length_percentage_auto_percent_to_taffy_not_normalized` は `..._is_normalized` に改名）、実在しない「proposals.md P49」を引く module doc の 2 行とテストの doc を消した。ワークスペースの中で `LengthPercentage(Auto)::Percent` を使う本番の呼び手は 0（`git grep -n -E "LengthPercentage(Auto)?::(Percent|percent)"` は定義とこのテストだけ）。較正: 期待を先に裏返して本番が `/ 100.0` を持たない形（直す前の本番そのもの）で `cargo test -p wintf --test layout dimension_conversion` を回し、`test_length_percentage_auto_percent_to_taffy_is_normalized`・`test_length_percentage_to_taffy` の 2 本が赤（`percent(50.0)` と `percent(0.5)` の食い違い・`percent(25.0)` と `percent(0.25)`）になるのを見てから本番を直して緑にした。94 は `src/ecs/layout/box_style.rs` の `FlexContainer`・`FlexItem`（と `Default`）、`tests/layout/taffy_layout_integration_test/box_components.rs` と `mod box_components;` を消し、`src/ecs/layout/mod.rs` の `LayoutRoot` の doc の例（`ignore`）から `FlexContainer::default(),` の行を消した。101 は `src/ecs/layout/metrics.rs` の `Opacity`（doc の例ごと）と 1 行目の allow の注記・`#![allow(deprecated)]` を消し、`tests/layout/metrics_test.rs` は煙（`validate` の 2 本）・derive（`PartialEq`）・`Opacity` の 4 本と allow を消して、`LayoutScale`・`TextLayoutMetrics` の既定値の 2 本だけにした。`Opacity` を比べの相手にしていた `tests/visual/component_test.rs` の `test_visual_clamped_opacity_matches_opacity_clamped` は、`value.clamp(0.0, 1.0)` と比べる `test_visual_clamped_opacity_clamps_to_unit_range` に書き換えた（範囲外の丸めを見るのはこの 1 本だけなので消さない・G4 の持ち物のファイルの 1 本の追従）。93 は `wrapper_types.rs` と `mod wrapper_types;` を消した。52 は `unit_integration.rs` の L5-L76・L92-L208・L362-L376・L393-L401・L409-L418 を消し（7.5 の `test_dimension_const_constructors` と 7.9 の本体は残す）、7.9 の終わりの `TODO`（消した `FlexItem` を引く）を「高さは親の Flex レイアウトが決めるので確かめない」の 1 行にした。`taffy_layout_integration_test/mod.rs` は使わなくなった import と消したファイルの説明を落とした。103 は `mapping_management.rs` の `test_mapping_consistency_verification` を、2 つ作って 1 つ消した後に `verify_mapping_consistency()` を呼ぶ形にした（52 で消える `unit_integration.rs:L54-L66` の 1 本をここへ寄せた）。較正: `TaffyLayoutResource::remove_node`（`src/ecs/layout/taffy.rs`）の `self.node_to_entity.remove(&node_id);` を外すと `test_mapping_consistency_verification` が赤（直す前の空の登録簿の形は緑のまま）になることを確かめてから戻した（戻した後は HEAD と同じ）。消す前と後に `git grep -n -w -e FlexContainer -e FlexItem -e Opacity -e wrapper_types -e box_components -e P49 -- . ':!vendors' ':!.kiro/specs'` を引いた。消した後の残りは型の参照ではない次だけ: example の文字列・注記（`dcomp_taffy_demo.rs`・`taffy_flex_demo/`）、Visual の不透明度を一般の語として引く注記（`ecs/graphics/systems/visual_sync.rs`・`ecs/layout/hit_test/`・`ecs/world/schedule_labels.rs`・`tests/layout/component_conversion_test.rs`・`taffy_flex_layout_pure_test.rs`）、`ecs/layout/systems/taffy_systems.rs:18` の「旧: 8コンポーネント」の昔の一覧（今の形の説明ではないので残す）、`doc/archive/WinVisual.md` の古い擬似コード。`ecs/graphics/visual.rs:140` の「`Opacity::clamped()` の移行先」は表の 101 のとおり `audit-wintf-graphics-world` の持ち物で、タスク 7.8（G5 の言及の掃除）で直す。戻した後は `cargo build --workspace --all-targets` が通り、`cargo test -p wintf --test layout` が 143 本、`cargo test -p wintf --test visual component_test` が 13 本とも緑。

#### 繰り延べた調べの答え

- **15 番（WIC の α）**: `windows` 0.62.2 は `IWICImagingFactory::CreateComponentInfo`（`Imaging/mod.rs:3486`）と `IWICPixelFormatInfo2::SupportsTransparency`（`:5614`）を持ち、機能 `Win32_Graphics_Imaging_D2D`（根の `Cargo.toml:74`）が既に入っている＝依存も機能も足さない。WIC の文書は「番号つきの形式は透明度を持ちえても TRUE を返さない」と言うので、パレットの道（`palette_has_alpha`）は残る。手書きの 12 の一覧には `16bppBGRA5551`・`64bppRGBAHalf`・`40bppCMYKAlpha` などの α つきの形式が入っていない（TIFF・JPEG XR の読み手が返しうる）。
- **25・34・49・50・53 番**: 本番を読んで、確かめを直しても緑の見込みと判断した。25＝`ecs/common/tree_system.rs:54-79` は bevy の写しで祖先へ `set_changed` を伝える。34＝`ecs/graphics/systems/init.rs:117-120` は有効な装置なら早く戻る。49＝`ecs/layout/systems/window_pos_systems.rs:52-59` に大きさ 0 以下の守りがある。50・53 も守りがある（ギャップ分析の行）。実際には直したテストを回して決める（下の「決め」）。
- **45 番**: `tests/graphics/common/mod.rs` の説明（別のスレッドに 2 つ目の MTA の Compositor を作ると落ちる・同じスレッドで作り直すのは安全）は、完了した `wintf-gpu-test-crash` の根本原因の記録が正本で、統制実験で確かめられている。`tests/visual/common/mod.rs` の「慣らしの描画面で DispatcherQueue が安定する」は実測の回避で、持ち主のスレッドは常駐の DispatcherQueueController を 1 つ持つので、こちらの説明の効き目も含む。持ち主のスレッドへ寄せれば、どちらの説明が正しくても落ちる形が無くなる。
- **30 番**: 本番の口は `audit-wintf-graphics-world` が持つ（requirements の「指摘の扱い」）。こちらは古いテストを消すだけ。
- **84 番**: `w!("Static")` は `crates/wintf/src` の 19 ファイル。範囲の中は `api.rs`・`api_owner_chain_probe_tests.rs`・`win_style.rs`（2 か所）・`win_style_accept_files_tests.rs` の 4 ファイル 5 か所。残りの 15 ファイルは `ecs/clickthrough/`・`ecs/window/`（ほかの見直しの持ち物）。
- **2・8 番**: `vsync_loop` の形は `wintf-dwmflush-wait-compensation` の brief の Approach をそのまま写す。

#### そのほかに確かめたこと

- **66 番**: crates.io の keywords は 1 語 20 文字まで。`windows-ui-composition` は 22 文字で入らないので、`directcomposition` を外すだけにする（4 語になる）。
- **3 番**: `to_window_coords_for_creation`（`ecs/window/window_pos/mod.rs:362-380`）は、片方が既定値のとき両方を素通しで返し、その形は `ecs/window/window_pos/tests.rs:234` が固定している。直しは生成の側（`window_factory.rs`）だけで足りる（与えられた側だけを `SWP_NOMOVE`／`SWP_NOSIZE` で反映）。枠の厚みの換算を片側にも効かせるのは、その固定を変えるので、この spec では変えない。
- **6 番**: `create_windows`（`ecs/window/window_system.rs:35`）の問い合わせは `With<Window>, Without<WindowHandle>`。失敗の印を 1 つ足して問い合わせから外す。`Window` を外すと窓の片付けのフックが動きうるので採らない。
- **65 番**: `logging.md:118` の差し替え先は、`tracing_subscriber` を立てる example のうち同じ節で使っていない `tooltip_demo`。
- **依存を外すと**: `Cargo.lock` から `nonmax`・`ambassador`（と、それだけが引く木）が消える。`THIRD-PARTY-NOTICES.md` は完了の全体テストが作り直す。C17 の約束は「足さない」だけで、C17 で `Cargo.lock` を触るのはこの spec だけ。
- **`crates/wintf/` は `doc/`・`docs/` を名指す行が 0**（`git grep -n "doc/\|docs/" -- crates/wintf`＝0 件）。直しでも足さない。

### 設計の決め

#### 決め: 15 番は `SupportsTransparency` に置き換え、emo-atlas の写しも同じに直す
- **背景**: 本番の唯一の呼び手が `has_alpha` を捨てる。手書きの一覧が 2 か所にある。
- **案**: ① `has_alpha` ごと消す ② 手書きの一覧を `IWICPixelFormatInfo2::SupportsTransparency` に置き換える ③ 今のまま
- **採った案**: ②。wintf の `pixel_format_has_alpha` を「工場から形式の情報を取り `SupportsTransparency` を聞く」に替え、areka-emo-atlas の `decode/wic_arm.rs` の写しも同じに替える。パレットの道（番号つきの形式）は両方とも今のまま。
- **理由**: ① は完了した `emo-atlas-palette-alpha` の要件 3（wintf の読み込みも読む前の α の有無を返す・emo-atlas と同じ答え）を覆す（要件 4.5）。② はその spec の「写しを保ち、同じ答えをテストで見張る」の決めのまま、手書きの一覧（ずれていくもの）を無くし、一覧から漏れていた α つきの形式を正しく答える。片方だけ替えると 2 つの読み手の答えが分かれる（同じ spec の要件 3.5）ので、emo-atlas の写しも替える（`audit-emo-atlas` は C19 の見込みで今は走っていない）。
- **残るもの**: 本番の呼び手が `has_alpha` を捨てる無駄（番号つきの絵でだけパレットを 1 回写す）は、上の要件 3 が求める形なので残す。完了した spec の要件 3.3（パレットでない形式の答えを今と変えない）は、その spec の直しが巻き添えで答えを変えないための約束と読み、今回の直し（漏れていた形式を正しく答える）はそれを覆さないと判断した。設計の討議で確かめる。
- **判定の規則（実測が要る所）**: 実装のとき、今の一覧の 12 の形式に `SupportsTransparency` が TRUE を返すことをテストで確かめる。どれかが FALSE なら、その形式だけを名前つきの定数に残して「WIC の答え または 定数」にし、理由を 1 行の注記に書く。

#### 決め: 29 番は、見本が無いときだけ 6 本のテストを「無視」にする（見本は写さない）
- **背景**: wintf のテストが areka-emo-atlas の中の見本を相対パスで読むので、ワークスペースの外（crates.io の荷）では 6 本とも落ちる。
- **案**: ① 6 つの見本を wintf へ写す（ギャップ分析の案） ② 見本の在る無しを `build.rs` で調べて設定（cfg）を立て、無いときは 6 本を `#[ignore]` にする ③ 今のまま（⑴ として残す）
- **採った案**: ②。`build.rs` が `CARGO_MANIFEST_DIR/../areka-emo-atlas/src/testdata/palette` の在る無しを見て `cfg(areka_palette_samples)` を出す（`rustc-check-cfg` で名を宣言する）。6 本は `cfg_attr(not(areka_palette_samples), ignore = "...")`。
- **理由**: ① は 2026-10-31 の設計の討議の議題 1（見本は 1 か所・写しのずれを見張るものが無いので二重に持たない）を覆す（要件 4.5）。② は見本を 1 か所のまま、ワークスペースの中では今と同じに 6 本が回り、外では「無視」と名乗って黙って緑にならない。
- **残るもの**: wintf を組むたびに `build.rs` が 1 つの道の在る無しを見る（読むだけ）。
- **確かめ**: ワークスペースの中で 6 本が「無視」にならずに回ること（テストの結果の数で見る）。

#### 決め: 25・34・49・50・53 番は「直したテストを回して決める」
- 直したテストが緑なら終わり。赤なら、今の本番の後退が初めて見えたということ。そのときは赤のテストを `#[ignore]` で隠さず、新しい指摘として開発者に報告し、本番の直しの持ち主を決めてもらう（要件 8.2 の扱い＝要件 3 の裁定を経る）。本番を読んだ見立ては 5 件とも緑。

#### 決め: 45 番は見た目のテストを全部 GPU の持ち主のスレッドへ寄せ、慣らしの描画面を外して確かめる
- `tests/visual.rs` に `#[path = "graphics/common/mod.rs"] mod gpu;` を足して同じファイルを使う（写さない）。27 本の本体を `gpu::on_gpu_owner_thread(|| ..)` で包み、`visual/common/mod.rs` の `init_com_mta` と慣らしの描画面を消す。
- **判定の規則**: 実 GPU で見た目の実行ファイルを 2 回回して 2 回とも緑なら確定。落ちる（プロセスの死を含む）なら、慣らしの描画面を持ち主のスレッドの中に戻して同じく 2 回回し、緑なら「慣らしが要る」を `visual/common/mod.rs` の注記に 1 行で残す。それでも落ちるなら止めて報告する。
- 自己テスト（`fixture_self_tests` のうち 114 番の後に残る 1 本）は 2 つの実行ファイルで回る。数 ms で、写しを持つより安いので受け入れる。

#### 決め: 84 番の試験の窓の補助は `src/api.rs` に置く
- `api.rs` の `#[cfg(test)]` に `create_test_window(ex, style, parent, title) -> HWND` と `with_test_window(ex, style, parent, f) -> R`（作る・`catch_unwind` で本体を回す・必ず壊す・panic を投げ直す）を置く。今の `api.rs` のテストの中の `create_test_window`（`:370-399`）を外へ出して、拡張の形の引数を足す。
- 使う所: `api.rs` のテスト・`win_style.rs` のテスト 2 本（18 番の後に残る `apply_click_through`・`apply_layered_companion`）・`win_style_accept_files_tests.rs`・`api_owner_chain_probe_tests.rs` の `create_probe_window`（作った後に `SW_SHOWNOACTIVATE` で見せる所だけ自分で持つ）。
- 理由: 範囲の中に置けて、`ecs/test_support.rs`（あちらの持ち物・形を変えない約束）に触らない。`ecs/` の 15 ファイルはそれぞれの見直しが使うかを決める。`audit-wintf-graphics-world` へ知らせる。
- 調べの窓の位置: 今の `(10,10)` を `(0,0)` にそろえる（大きさ 0×0 なので重なりの順の答えに効かない）。調べのテストが緑のままなことで確かめる。

#### 決め: 刻みの土台の名前の変更は、あちらの着地の後にまとめる（72・73）
- `AsyncTickTask`・`MessageLoopDriver` を消し（73）、`notify_shutdown` を `request_exit` へ畳む（72）と、`ecs/world/`・`ecs/app.rs` の注記に古い名前が残る。そのファイルは `audit-wintf-graphics-world` の持ち物で、あちらは 13 番で `try_tick_on_vsync` を消すか名前を変える。両方を、あちらの着地の後に `tick_bridge.rs` の説明の行（申し合わせでこちらの持ち物）と合わせて 1 回で直す。

### まとめ方（synthesis）

- **まとめ（一般化）**: 「試験の窓を作って必ず壊す」（84）・「GPU の持ち主のスレッドで回す」（45・113）・「1 回分の処理に時間と待ち方を差し込む」（2）は、どれも今ある形を 1 か所に寄せるだけで、新しい抽象は作らない。
- **作るか使うか**: α の判定は手書きの一覧をやめて WIC の `SupportsTransparency` を使う（15）。待ちの補いは `std::thread::sleep`（brief どおり）。ログの確かめは既存の `log-capture-kit` の `count_levels`。試験の口は既存の `drain_window_pos_commands()`（39）・bevy の `DeferredWorld::from`（36）・WUC の `Visual::Opacity()`／`Offset()`／`ContainerVisual::Children()`（40・41）。
- **減らす**: 115 件のうち約 7 割が「消す」。新しく足す本番のコードは、`compensation`・`vsync_once`・`join_vsync`（2・8）・`initial_pos_flags`・生成の失敗の印（3・6）・`init_ui_com`（7）・中継の関数（11）・塊の数の問い合わせの判定（4）・`build.rs` の数行（29）だけ。

### 危うさと手当て

- 直したテストが今の本番の後退を初めて見せる（25・34・49・50・53）→ 上の「回して決める」。
- 見た目のテストを持ち主のスレッドへ寄せると直列になり時間が延びる → 27 本は小さく、graphics の実行ファイルで既に同じ形。実 GPU で 2 回だけ回す（高負荷テストを何度も回さない）。
- `audit-wintf-graphics-world` の着地が遅れる → 後回しの組だけが待つ。先の組は独立に進む。
- 公開の口を消す → 一覧（`design.md` の「公開の口の変化」）を次の版の知らせへ。`#[deprecated]` は挟まない（討議の裁定 3）。

## 12. 設計の討議の結果（2026-11-05）

- 開発者「承認」で 6 議題を推奨案で確定した（正本は `design.md` の「設計の討議の裁定」）。
- 設計の検証の指摘 1 は自明な直し（`create_command_list` の 4 か所目と組の順）、指摘 2 は議題 1（完了した spec の要件 2.3・3.3 の狭い改め）、指摘 3 は議題 4（印を `pub` にして `wintf::WindowCreationFailed` として出す）で片づけた。
- `audit-wintf-graphics-world` の追加の申し合わせで、T2a・T2b と `create_command_list` の削除をあちらの着地の後へ回した。
