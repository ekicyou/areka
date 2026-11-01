# Brief: menu-thumbnail-preview

> 出どころ（2026-10-20 開発者・`/kiro-discovery`）: 「サムネイル表示に対応する。メニューでゴーストを選択中、サムネイル画像を表示する機能がある（ukadoc参照）arekaが対応している画像形式について、対応してほしい。α付き画像の透過も対応が欲しい。対応時期はdolaとかの基盤対応が終わってからで良いかな。」
> 同日の答え: 作り方＝**メニューとは別の小さな窓で出す**（オーナードローの裁定を待たない）・版＝**v0.4.0**・動く絵（APNG・動く WebP）のサムネイル＝**動かして見せる**。

## Problem

利用者は、右クリックメニューでゴースト・シェル・バルーンを選ぶとき、名前だけを見て選ぶしかない。SSP は、メニューの項目にカーソルを乗せている間、その項目の `thumbnail.png` をメニューの横に出す（SSP ヘルプ「設定：表示」の「メニュー選択中にプレビュー画像を表示する」＝ゴースト切換え・呼び出し・シェル切換え・バルーン切換え等）。作者もサムネイルを同梱して配っている。areka ではそれが一度も見えない。

正典（ukadoc「ゴースト」の `thumbnail.png`）: 「メニューツリーでゴーストの選択時に表示されるサムネイル」。形式は png か jpg。`.pnr` に名前を変えると、左上の 1 点（座標 0,0）と同じ色が抜ける。シェル（ukadoc「シェル」）とバルーンのフォルダにも `thumbnail.png` を置く。

## Current State

- **メニュー**: Win32 標準の `TrackPopupMenuEx`（`crates/areka/src/menu/win32.rs:29-69`・`TPM_RETURNCMD`）。メニューが閉じるまで戻らず、選ばれた番号だけを返す。持ち主の窓はキャラの窓。メニューを出している間も tick は回る（`menu/trigger.rs` の `display`・要件 7.2）。
- **カーソルが乗った知らせを受ける所が無い**: `WM_MENUSELECT`・`WM_INITMENUPOPUP`・`WM_EXITMENULOOP` はどこでも受けていない。窓の知らせは wintf の `crates/wintf/src/ecs/window_proc/mod.rs` の `dispatch_window_message` を通り、表に無いものは `DefWindowProcW` へ落ちる。
- **項目の番号から先の引き方**: `menu/plan.rs` が 1 回の表示の中で番号を振り、`MenuPlan.actions` が「番号 → (Frame, MenuAction)」を持つだけ。`MenuAction` は閉包で、フォルダの名前を中に閉じ込めている（`ghost_frame.rs`・`shell_frame.rs`・`balloon_frame.rs`）。番号からサムネイルの場所を引く表は無い。各段の供給者は一覧の `entry.dir` を持っているが、捨てて `identity` だけ使う。
- **サムネイルは「在るか」だけを見ている**: `crates/areka-ghost/src/catalog.rs` の `THUMBNAIL_FILE = "thumbnail.png"`・`Identity.has_thumbnail`（`top.join(THUMBNAIL_FILE).is_file()`）。絵は読まない。ゴーストの `top` はゴーストの最上位のフォルダで、ukadoc の書く `ghost/master/` ではない（どちらが SSP の動きかは要件の段で測る）。`.pnr` は読まない（完了 `baseware-root-layout` の要件で先送りと明記）。既定のバルーン StayseeBalloon は `thumbnail.pnr`（176×43・パレット）だけを持ち、`.png` が無い＝今は「サムネイル無し」の扱い。（2026-10-30 から既定は `areka-balloon`。`StayseeBalloon` は配布物に同梱する選べるバルーン。`areka-balloon` は `thumbnail.*` を持たない）
- **絵を読む道はもう在る**: 止まった絵は WIC（`areka-emo-atlas/src/decode/wic_arm.rs`・中身で形式を決める＝png・jpg・bmp など）、動く絵（APNG・WebP）は `image` クレート（`png`・`webp` の機能だけ・`decode/image_arm.rs`）。α は `Normalizer`（`normalize.rs`）が決める＝`UseSelfAlpha::Off` が `.pnr` の「左上の 1 点の色を抜く」とちょうど同じ。動く GIF は扱わない（決めごと）。
- **小さな絵の窓の作り方**: wintf の ECS の窓（`WS_POPUP`・`WS_EX_TOOLWINDOW|WS_EX_TOPMOST`・DirectComposition で画素ごとの α）＋子の `BitmapSource` の例が `crates/areka/examples/mock-shell.rs` に在る（本番では使っていない・抜き色の扱いは無い）。

## Desired Outcome

- 右クリックメニューのゴースト・シェル・バルーンの切り替えの項目にカーソルを乗せている間、その項目のサムネイルがメニューの横に出る。別の項目へ移れば絵が替わり、サムネイルの無い項目・メニューの外・メニューを閉じたときは消える。
- areka が読める形式（png・jpg・APNG・WebP・ほか WIC で読める止まった絵）のサムネイルが出る。動く絵は動く（再生は dola の時刻で回す）。
- α 付きの絵は画素ごとの α で透けて出る。`.pnr` は左上の 1 点の色を抜いて出る（譲歩として正典どおり）。既定のバルーンの `thumbnail.pnr` も出る。
- メニューは Win32 標準のまま（生きている決まり 6 を変えない）。サムネイルはメニューと別の窓なので、`popup-menu-shell-look` の裁定がどちらに転んでも並んで働く。
- 画面の DPI に追従した大きさで出る。

## Approach

選んだ案＝**メニューとは別の小さな窓で出す**。キャラの窓（メニューの持ち主）に届く `WM_MENUSELECT` を wintf の `dispatch_window_message` で受け、項目の番号を areka へ渡す。メニューの組み立て（`plan.rs`）で「番号 → サムネイルの場所」の表を `actions` と並べて作り、番号が引ければ、そのサムネイルを読み（既存の WIC／`image` の道と `Normalizer`）、メニューの横に wintf の窓（DirectComposition・画素ごとの α・前面・活性にしない）で出す。メニューの表示中も tick は回るので、窓の出し入れと動く絵のコマ送りは tick の側で行える。

- **採らなかった案**: オーナードローのメニューの項目の中に描く（`popup-menu-shell-look` の裁定〔生きている決まり 6〕を待つことになり、SSP とも見た目が違う）。素の Win32 の重ね窓（`UpdateLayeredWindow`）で直に描く（wintf を通らず、窓の作りが二系統になる）。
- **版**: v0.4.0（dola の整備〔v0.2.0〕の後）。動く絵のサムネイルを dola の時刻の上で回すため、時刻の土台が固まってから作る。

## Scope

- **In**: `WM_MENUSELECT`（とメニューの終わりの知らせ）の受け口・番号からサムネイルを引く表（ゴースト切り替え・シェル切り替え・バルーン切り替え。この spec の着手の時点で在る「呼び出し」の段〔`ghost-call`〕も）・サムネイルの名前の探し方（`thumbnail.png`・`.pnr`・`.jpg` と、areka が読める形式の名前をどの順で探すか）・α と `.pnr` の抜き色・動く絵の再生・メニューの横に出す窓（位置・DPI・画面の端での置き直し）・読めなかったときの記録（絵を出さず、黙らずに記録を残す）・決定論のテストと実機での確かめ。
- **Out**: エクスプローラーの上でのホバーのプレビュー（SSP の設定の [✓] の段）・表示を切る設定（SSP の「メニュー選択中にプレビュー画像を表示する」の切り替え＝設定画面〔windows-reactor 待ち〕の持ち物）・オーナードローと着せ替えのメニュー（`popup-menu-shell-look`）・プロパティ `thumbnail` の値（`property-catalog-lists`）・`OnRecommendsiteChoice` の `thumbnail_url`・`.pna` の α（決めごと＝無視する）・動く GIF（決めごと＝扱わない）。

## Boundary Candidates

- メニューの側（areka の `menu/`）: 番号 → サムネイルの場所の表、知らせを受けたときの振り分け。
- wintf の側: `WM_MENUSELECT` などを `dispatch_window_message` で受けて ECS へ流す口（汎用の口として。areka 固有の意味を持たせない）。
- 絵の側: サムネイルを読んで α を決める（既存の atlas の読み手と `Normalizer` を使い回す・新しい読み手を作らない）。
- 出す窓の側: wintf の ECS の窓＋絵（動く絵のコマ送りを含む）。

## Out of Boundary

- 右クリックメニューそのものの見た目と並び（`popup-menu-shell-look`・`popup-menu-residue`）。
- 一覧の作り方（`areka-ghost` の catalog）。`has_thumbnail` の名前の探し方を広げる必要があれば、ここから直す（`.pnr` などを数える）が、一覧の形は変えない。
- 設定画面と設定ファイル（予約・windows-reactor 待ち）。

## Upstream / Downstream

- **Upstream**: v0.2.0 の dola の整備（`dola-interrupt-ticks` ほか・時刻の土台）と wintf の整備（`wintf-d2d-composite-rules`）。メニューの土台（完了 `popup-menu-minimal`）・一覧（完了 `baseware-root-layout`・`baseware-root-list`）・動く絵の読み手（完了 `animated-image-decode`・`animated-image-playback`）。
- **Downstream**: `ghost-call`（呼び出しの段のサムネイル）・設定画面（表示を切る設定の入口）・`popup-menu-shell-look`（オーナードローになっても同じ知らせの口を使う）。

## Existing Spec Touchpoints

- **Extends**: なし（新しい境目）。
- **Adjacent**: `popup-menu-shell-look`（同じ `window_proc/mod.rs`・`menu/{mod,win32,plan}.rs` を触る＝同じウェーブに置かない）・`popup-menu-residue`（`menu/` を触る・相乗りが安い）・`ghost-call`（呼び出しの段を足す）・`property-catalog-lists`（プロパティ `thumbnail`）・`audit-areka-input-menu`・`audit-ghost-catalog`（v0.1.0 の後の隙間の見直しで同じ所を直す＝先に着地する見込み）。

## Constraints

- メニューは Win32 標準のまま（生きている決まり 6）。
- α 付きの絵を正しく使うのが原則・抜き色（`.pnr`）は譲歩・`.pna` は無視・作者のバグは繕わない（決めごと）。動く絵は APNG と WebP の 2 形式だけ（動く GIF は扱わない）。
- 時刻は正確に扱う（動く絵の待ち時間を丸めない・遅れたら過ぎた分だけ進める）。
- DPI に追従する（拡大は D2D の変換行列）。
- 失敗は黙らない（読めない絵は記録を残し、絵を出さないだけで、メニューは止めない）。
- ukadoc で読めない点（ゴーストのサムネイルを置く場所〔最上位か `ghost/master/` か〕・探す名前の順・メニューの横に出す位置・`.jpg` の名前）は、議題にする前に SSP の MCP で測る。
- 規模の見立て: M（10〜14 タスク）。

## 2026-11-01 棚卸㉝の後の開発者の決め

2026-11-01 開発者の承認（棚卸㉝の後・ツールチップは「境界領域です。リアクター描画にした方が良い可能性がある。WinUIで描画できる要素ですからね」）＝v0.4.0 から windows-reactor 待ちの段へ移した。メニューに凝ったことをする仕事で、リアクターが来ればメニューは作り直す（2026-10-31 開発者「メニュー関係はreactor解放されたら大改造する」）。
