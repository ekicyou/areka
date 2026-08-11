# ギャップ分析: areka-P0-ghost-window-zorder

> 実施 2026-08-11（`/kiro-validate-gap`）。対象は確定済み `requirements.md`（要件 1〜8）と、
> **`file-slimming`（PR#103）マージ後の現物**（本ブランチ `claude/ghost-window-zorder-0055fb`・
> 起点 `fdddef4`）。本書のアンカー（file:line）は全て本セッションで実測し直したものである。
> brief.md の追記(58) が挙げるアンカーの現況は §1.7 に照合表を置いた。

## 0. 要旨

- **語彙は完備・配線ゼロ**という brief の診断は現物で成立する。`ZOrder`（`NoChange`/`TopMost`/
  `NoTopMost`/`Top`/`Bottom`/`InsertAfter(HWND)`）とビルダー・フラグ変換・単体テストは
  `wintf` に揃っており、公開エクスポートもある。しかし**本番で `NoChange` 以外を指定する
  コードは 1 箇所も無い**。
- ただし brief の想定より**配線先が 1 本ではない**。ゴースト窓の位置書込は areka 側の
  単一 funnel `enqueue_window_set_pos` が担い、そこは `SWP_NOZORDER` と
  `hwnd_insert_after: None` を**ハードコード**したうえで `WindowPos` を
  `bypass_change_detection` で書く。ゆえに `WindowPos.zorder` に値を入れても
  **この経路では一切効かない**（wintf 側の `apply_window_pos_changes` は
  `Changed<WindowPos>` 駆動で、この経路では発火しない）。**z-order を効かせる配線は
  2 経路のどちらか（または両方）に明示的に足す必要がある**。
- **活性化を ECS から観測する面が存在しない**。`WM_ACTIVATE` ハンドラは在るが
  **非活性化（`WA_INACTIVE`）だけを処理して早期 return** しており、活性化側は素通し。
  かつ配送表は `pub(crate)`、ハンドラは wintf 内、スコープ↔窓の対応（`GhostWindows`）は
  areka 内で、**wintf → areka の import は設計規約で禁止**。したがって「どのスコープが
  活性化したか」を areka へ渡すシームの新設が、案 B 系の実質的な主作業になる。
- **案 A（Win32 owner）は現行の窓生成経路では素直に張れない**。生成はライブラリ
  `wintf-winmsg-executor` の `LibWindow::new_ex(WindowType::TopLevel, …)` で、
  **hWndParent を受け取らない**。既存の `Window.parent` は生成後 `SetParent`＝**子窓**化で
  owner ではない（コード自身がその非等価を明記）。owner を張るなら
  `SetWindowLongPtrW(GWLP_HWNDPARENT)` の生成後適用か、ライブラリ側 API の拡張が要る。
  加えて **owner の破棄は owned を巻き込む**ため、**要件 5.7 と正面から衝突する**。
- **実機サインオフは画素を見なくても機械判定できる**。GPU 合成窓はスクリーンショットが
  効かない既知制約があるが、**z-order そのものは `GetWindow(GW_HWNDPREV/GW_HWNDNEXT)`／
  `GetTopWindow` で OS に問い合わせられる**。「バルーンがキャラの直前にいる」を実 HWND 列で
  検証してログに落とせば、要件 7.2〜7.5 は有界時間の自動終了＋ログ突合で判定できる。

---

## 1. 現状調査（現物・file:line）

### 1.1 ゴースト窓の生成

- `crates/areka/src/placement/spawn.rs:217-328` `spawn_ghost_windows`
  — スコープごとに **バルーン窓 → キャラ窓の順**で spawn する（`BalloonFollow.balloon` が
  entity を要するため・`:234`／`:262`）。両者とも同型の `WindowStyle`／`WindowPos`／
  `HitTest::none()`／`DpiSuggestedRectPolicy::ExternalAuthority` を持つ。
- `crates/areka/src/placement/spawn.rs:332-337` `window_style()`
  — `WS_POPUP | WS_VISIBLE` ／ `WS_EX_LAYERED | WS_EX_TOOLWINDOW`。
  **`WS_EX_TOPMOST` を含めない**（DD13）＝**要件 8.1「常時最前面ではない通常の窓」は現状すでに成立**。
  `WS_EX_TOOLWINDOW` により**タスクバー・Alt+Tab 非露出も既に成立**（要件 5.5／8.5 の現状基盤）。
- `crates/wintf/src/runtime/window_factory.rs:99-201` `EcsWindowFactory::create_window`
  — `LibWindow::new_ex(WindowType::TopLevel, ex_style, state, wndproc)`（`:137`）で生成。
  **親／owner の HWND を渡す口が無い**。`ex_style` は `compute_ex_style`（`:64-68`）で
  `WS_EX_LAYERED` を落とし `WS_EX_NOREDIRECTIONBITMAP` を付与（GPU 合成固定）。
- `crates/wintf/src/runtime/window_factory.rs:149-165`
  — `Window.parent`（`ecs/window/components.rs:97-102`）は生成後 `SetParent` で反映する。
  **コメント `:154-157` が「`CreateWindowExW` の hWndParent は非子窓では owner を意味するので
  厳密一致しない」と自ら明記**している。現行 areka／全 example は `parent: None`。
- 生成後初期化（`:209-269`）の `SetWindowPos` は 2 回とも `SWP_NOZORDER | SWP_NOACTIVATE`。

### 1.2 z-order の語彙（実装済み・未配線）

- `crates/wintf/src/ecs/window/window_pos/mod.rs:25-39` `enum ZOrder`
  （`NoChange`/`TopMost`/`NoTopMost`/`Top`/`Bottom`/`InsertAfter(HWND)`）。
  `unsafe impl Send/Sync`（`:49-50`）で ECS コンポーネントに載る。
- ビルダー `:119-158`（`with_zorder`／`zorder_topmost`／`zorder_top`／`zorder_bottom`／
  `zorder_insert_after` ほか）。
- `build_flags`（`:233-283`）は **`zorder == NoChange` のときだけ `SWP_NOZORDER` を立てる**（`:246-249`）。
- `get_hwnd_insert_after`（`:293-302`）が `ZOrder` → `Option<HWND>`（`HWND_TOPMOST` 等の
  疑似ハンドル含む）へ変換する。
- 単体テストは `window_pos/tests.rs:82-221` に既に揃っている
  （ビルダー・`SWP_NOZORDER` の付け外し・`get_hwnd_insert_after` の全腕写像）。
- 公開エクスポート: `crates/wintf/src/ecs/mod.rs:49` に `ZOrder` が載っており **areka から使える**。
- **本番指定はゼロ**: `ZOrder`／`zorder_*`／`InsertAfter` の非テストヒットは wintf 内部の
  定義・変換・エクスポートのみ。areka 側のヒット
  （`placement/config.rs:72` ほか）は descript の `seriko.zorder`＝**SERIKO レイヤ順**で
  **本 spec の窓 z-order とは別概念**（brief 追記(58) の混同予防どおり・編集対象ではない）。

### 1.3 `SetWindowPos` の実際の経路（**2 系統ある**）

| # | 経路 | 実体 | z-order 引数 |
|---|---|---|---|
| ① | wintf `apply_window_pos_changes` | `graphics/systems/window_pos.rs:21-109` | **`WindowPos.zorder` を尊重**（`:71-72` で `build_flags_for_system`／`get_hwnd_insert_after`、`:89-98` で `SetWindowPosCommand` へ） |
| ② | areka `enqueue_window_set_pos` | `placement/follow/window_move.rs:452-544` | **`SWP_NOZORDER` 固定**（`:484-487`）＋**`hwnd_insert_after: None` 固定**（`:496`） |
| ③ | wintf ドラッグ移動 | `window_proc/mouse_move.rs:438-450` | `SWP_NOSIZE\|SWP_NOZORDER\|SWP_NOACTIVATE`（`:447`） |
| ④ | wintf `WM_DPICHANGED` の提案位置適用 | `window_proc/window_pos.rs:407-434` | `SWP_NOSIZE\|SWP_NOZORDER\|SWP_NOACTIVATE`（`:428`）。ゴースト窓は `ExternalAuthority` ゆえ**そもそも通らない** |
| ⑤ | クリック透過のスタイル反映 | `win_style.rs:362-388`／`:401-424` | `SWP_FRAMECHANGED\|NOMOVE\|NOSIZE\|**NOZORDER**\|NOACTIVATE` |
| ⑥ | 生成後初期化 | `runtime/window_factory.rs:232-241`／`:251-260` | `SWP_NOZORDER`（＋`FRAMECHANGED` 等） |

- 実行体は `SetWindowPosCommand`（`ecs/window/command.rs:117-125`）で、
  **`hwnd_insert_after: Option<HWND>` を搬送できる**（`:124`・`new()` の第 7 引数 `:141`）。
  flush（`:173-206`）が `guarded_set_window_pos`（`:83-106`）へ渡す。
  **搬送路は既にあり、渡していないのは②③のみ**である。
- **重要な非対称**: ゴースト窓の実運用の位置書込はほぼ全て②であり、②は `WindowPos` を
  `bypass_change_detection` で書く（`:499-514`）。ゆえに **`Changed<WindowPos>` が発火せず
  ①は起動しない**。`WindowPos.zorder` にだけ値を入れる案は、**②を改造しない限り
  spawn 直後の 1 回しか効かない**。

### 1.4 活性化の観測面

- `crates/wintf/src/ecs/window_proc/keyboard.rs:119-169` `WM_ACTIVATE`
  — `:126-131` で **`activation_state != 0`（＝活性化）なら即 return**。現状の用途は
  ドラッグキャンセルのみ。**活性化側の席は完全に空いている**（brief の記述どおり）。
- 配送表 `crates/wintf/src/ecs/window_proc/mod.rs:43-71`（`WM_ACTIVATE` は `:70`）。
  関数は `pub(crate)`（`:33`）＝**決定論テストは wintf クレート内に置く必要がある**。
- `WM_ACTIVATEAPP`／`WM_MOUSEACTIVATE`／`WM_NCACTIVATE` は**配送表に無い**。
- `SetForegroundWindow`／`BringWindowToTop`／`SetActiveWindow`／`GetWindow`／
  `GWLP_HWNDPARENT` は **wintf・areka 双方に 1 件も無い**（実測 grep）。前面化は
  Windows の既定動作に委ねている。
- `WM_WINDOWPOSCHANGED`（`window_proc/window_pos.rs:36-272`）は在り、`lparam` の
  `WINDOWPOS` を読んでいるが、**`hwndInsertAfter`／`SWP_NOZORDER` は一切見ていない**
  （位置・寸のみ消費）。`is_self_initiated()`（`:44`）による自発呼び出し判定の仕組みは
  既にあるため、**z-order 変化を検知して再調整しても自己ループを断つ道具は揃っている**。

### 1.5 スコープ ↔ 窓の対応

- `crates/areka/src/placement/spawn.rs:163-201` `GhostWindows`（Resource）
  — `scope → ScopeWindows { char_window, balloon_window }`。
  `char_window(scope)`／`balloon_window(scope)`／`scopes()`／`remove_entry_of(entity)`。
- 逆引き marker: `CharWindowMarker { scope }`（`:85-88`）／`BalloonWindowMarker { scope }`（`:93-96`）。
- 共通標識 `GhostWindowMarker`（`:107-109`）に `on_remove` hook（`:122-142`）があり、
  片割れの despawn で scope エントリごと落ちる。
- **`GhostWindows` は areka 側**。wintf は areka を import できない
  （`ecs/window_proc/lifecycle.rs:35` が「**wintf → areka の import は禁止**」を明記）。
- 参考にできる既存パターン: `register_ghost_windows_click_through`
  （`spawn.rs:401-425`）が **`Added<WindowHandle>` で「HWND が付いた瞬間」を捉える** system。
  main.rs の `FrameFinalize` へ結線済み（`crates/areka/src/main.rs:687-692`・spawn は `:715`）。
  **owner／z-order の初期確立も同じ形（両窓の `WindowHandle` 出現後）でしか書けない。**

### 1.6 共存が要る既存性質（要件 5）

- 透過表示: 全窓 `WS_EX_NOREDIRECTIONBITMAP` の GPU 合成（WUC）固定。
- クリック透過: `WS_EX_TRANSPARENT` 動的トグル＋`WS_EX_LAYERED` 同伴フラグ
  （`win_style.rs:362-424`）。判定は α マスクで `clickthrough/controller.rs` が毎起床評価。
- ドラッグ: `DragConfig`／`OnDrag`／`OnDragEnd`。非 Free アンカーのキャラ窓は
  `move_window: false`＝areka 側が単一ライター（`spawn.rs:286-289`）。
- バルーン追従: `BalloonFollow { balloon, offset }`（`spawn.rs:291-294`）。
- 破棄: `WM_CLOSE`（`lifecycle.rs:97-116`）は `DestroyWindow` を直叩きせず
  `world.despawn(entity)` → `WindowRegistry` 要素 drop 駆動で `DestroyWindow`。
- **hide/show は現状ゼロ**（areka・emo-present に `ShowWindow`／`SW_HIDE`／`SWP_HIDEWINDOW`／
  `hide_window` の非テストヒットが 1 件も無い）。要件 2.6 は**未来の
  `areka-P0-balloon-visibility` に対する契約**であり、現状で発火する経路は無い。

### 1.7 brief 追記(58) アンカーの現況（`file-slimming` 後の再解決）

| brief の記載 | 現況 | 判定 |
|---|---|---|
| `ZOrder` :25-38 | `window_pos/mod.rs:25-39` | ほぼ一致（enum 末尾 +1） |
| ビルダー :119/:131/:156 | `:119`（`with_zorder`）／`:131`（`zorder_topmost`）／**`:155`**（`zorder_insert_after`） | 1 行差 |
| `window_pos/tests.rs:84-214` | `:82-221` | 微差・内容一致 |
| `keyboard.rs` WM_ACTIVATE :119 | `keyboard.rs:119` | **完全一致** |
| `window_factory.rs:152`（`parent: None`） | `:151-153` のコメント | 微差・内容一致 |
| `SetWindowPosCommand` の `hwnd_insert_after` :124/:141 | `command.rs:124`／`:141` | **完全一致** |
| `spawn.rs` `ExternalAuthority` :353-354・バンドル :245/:273 | `:353-355`／`:249`／`:275` | 微差・内容一致 |
| `NoChange` 以外の本番指定ゼロ | 現物でも**ゼロ** | 一致 |

> **追加の実測所見（brief に無い）**: 位置書込の単一 funnel `enqueue_window_set_pos` が
> `SWP_NOZORDER`＋`hwnd_insert_after: None` をハードコードしている点（§1.3②）は
> brief に登記が無い。**案 B の配線コストはここで決まる**ので、design で必ず織り込むこと。

---

## 2. 要件 → 資産マップ

凡例: ✅=現物で成立／🟡=部分的に資産あり（配線が要る）／❌=Missing／❓=Unknown（要調査）／⛓=Constraint

| 要件 | 必要な技術要素 | 現物の資産 | 判定 |
|---|---|---|---|
| 1.1/1.2 バルーンをキャラの直前へ | `ZOrder::InsertAfter(char_hwnd)` ＋適用経路 | 語彙 ✅／適用は①のみ・②は不可（§1.3） | 🟡 配線 |
| 1.3 バルーン活性化時にキャラを直後へ | 同上（対象を反転） | 同上 | 🟡 配線 |
| 1.4 全経路で反転させない | z-order を動かす**全経路の網羅**（活性化・OS 由来の raise 含む） | 活性化の観測面が無い（§1.4） | ❌ 新設 |
| 1.5 片方不在なら何もしない | `GhostWindows` の `Option` 返し／entity 生存確認 | `spawn.rs:172-179`・②の `world.get_entity(...).is_err()` 分岐（`window_move.rs:465-472`） | ✅ 流用可 |
| 2.1/2.3 ドラッグ完了時 | `OnDragEnd` フック | `spawn.rs:255`／`:312-314` | 🟡 呼出先を足すだけ |
| 2.2 別ディスプレイへの移動 | 同上（DPI 跨ぎは④が不通過ゆえ②のみ） | ② | 🟡 |
| 2.4 DPI 変化後 | `Changed<DPI>` → 再射影 → ② | `frame/dpi.rs`／`window_move.rs` | 🟡 |
| 2.5 復元・寸法変更 | `PlacementRoute::{Restore,KeepPositionResize,Resnap,…}` → ② | `follow/visibility.rs:178-192` に route 語彙 | 🟡 |
| 2.6 非表示→表示 | show/hide の実装 | **現状ゼロ**（§1.6）。`balloon-visibility`（W6 並走）の所有 | ⛓ シームのみ |
| 2.7 他アプリ後の再活性化 | 活性化の観測面 | 無し | ❌ 新設 |
| 3.1〜3.3 スコープ間を強制しない | 「2 窓だけ動かす」手段 | `InsertAfter` は当該窓 1 個のみ動かす＝**要件 3.4 を構造的に満たす**。`HWND_TOP` は満たさない | ✅ 手段選択で成立 |
| 3.4 他スコープに触れない | 同上 | 同上 | ✅ |
| 4.1/4.2 他アプリ活性化で一緒に沈む | 非 topmost・owner いずれかの OS 既定 | `WS_EX_TOPMOST` 無し（`spawn.rs:332-337`）＝**既定で沈む**。ただし**維持コードが「沈んだ後に勝手に持ち上げる」ことをしない**保証が要る | 🟡 反証責務 |
| 4.3 常時最前面にしない | `ZOrder::TopMost` を使わない | ✅ | ✅ |
| 4.4 背面でも相対順を保つ | `InsertAfter` は相対配置ゆえ保存される | ✅（設計選択に依存） | 🟡 |
| 5.1〜5.4 透過・クリック・ドラッグ・追従 | 手段が壊さないこと | ⛓ 案 A は**実機検証必須**（§3 案 A） | ❓ |
| 5.5 タスクバー／Alt+Tab | `WS_EX_TOOLWINDOW` 既設 | ✅（案 A でも owner 化で変化しない） | ✅ |
| 5.6 壊すなら代替手段 | 分岐の記録 | brief §Approach に記録済み | ✅ |
| 5.7 破棄の巻き込み禁止 | — | **案 A（owner）は OS が owned を巻き込み破棄する＝正面衝突**（§3 案 A） | ⛓ **重大** |
| 6.1〜6.3 診断ログ | `tracing` 構造化ログ・専用 target | `placement/diag.rs` の `DIAG_TARGET` 方式（`window_move.rs:558-580`）が先例 | ✅ 流用可 |
| 6.4 失敗しても継続 | `SetWindowPos` 失敗は warn+継続 | `command.rs:195-203` 既設 | ✅ |
| 7.1 決定論テスト | 判断を純関数へ切出し＋配送テスト | `dpi_helpers::dpi_suggested_position_decision` が**そのままの先例**（`window_pos.rs:370-403`）。ログ捕捉は `ecs::test_support::capture_under_filter` | ✅ 型がある |
| 7.2〜7.5 実機サインオフ | 実 z-order の観測手段 | **`GetWindow`／`GetTopWindow` は未使用＝新設**。ただし画素不要で機械判定できる（§4） | ❌ 新設（低コスト） |
| 8.1 既定は非 topmost | `WS_EX_TOPMOST` 無し | ✅ **既に成立** | ✅ |
| 8.2 `stayontop` を後から 1 ビットで | `ZOrder::TopMost` が既存語彙 | ✅ | ✅ |
| 8.3〜8.5 先送り語彙を実装しない | — | 現状未実装＝**何もしなければ成立**。「うっかり実装しない」だけ | ✅ |

---

## 3. 実装アプローチ

### 案 A: Win32 の owner 関係を張る（brief の推奨）

バルーン窓の owner をキャラ窓にし、「owned は常に owner より手前」という OS 保証で
要件 1・2 を**構造的に**満たす。

**現物で判明した実現手段の制約**:

1. 生成時に張れない。`LibWindow::new_ex(WindowType::TopLevel, …)` は hWndParent を取らない
   （`window_factory.rs:137`）。→ 選択肢は
   (a) 生成後 `SetWindowLongPtrW(hwnd, GWLP_HWNDPARENT, owner_hwnd)`、
   (b) `wintf-winmsg-executor`（`=0.0.5` 完全 pin のフォーク）への API 追加、
   (c) `Window.parent` の意味を owner へ変える（**既存 `SetParent` 経路の意味論変更**＝
   コメント `:154-157` が明示的に区別している契約を壊す。非推奨）。
2. **順序制約**: owner の HWND はバルーンの生成後に決まる（spawn 順は balloon → char・
   `spawn.rs:234`/`:262`）。ゆえに **`Added<WindowHandle>` で両窓が揃った後に張る**
   `register_ghost_windows_click_through` 型の system が要る（(a) を前提とする追加根拠）。
3. **要件 5.7 との衝突**: owner を `DestroyWindow` すると OS は owned を巻き込んで破棄する。
   現行の破棄は `despawn` → `WindowRegistry` drop → `DestroyWindow`（`lifecycle.rs:97-116`）で、
   **バルーンの HWND が二重破棄されうる**（ライブラリの `Window::drop` が既に無効な HWND へ
   `DestroyWindow` を撃つ）。要件 5.7 は「同一スコープの残る窓を破棄に巻き込んで消滅させない」
   と規定しており、**文面どおりなら案 A は違反する**。現実には対で死ぬ運用だが、
   **要件の文面 vs. OS 挙動の裁定が要る**（→ 設計判断 #4）。
4. **未知**: owner 化が `WS_EX_NOREDIRECTIONBITMAP`＋WUC 合成・`WS_EX_TRANSPARENT` 動的トグルと
   共存するか。**実機でしか判らない**（roadmap の W6 編成条件⑶が「案 A の WUC 共存実機検証が
   最初のタスク」と規定）。

**Pros**: 経路の網羅漏れが原理的に起きない／維持コードがゼロ／他アプリ活性化時の一括沈降も OS 任せ。
**Cons**: 副作用が広い（破棄・最小化・活性化の伝播）／導入手段が (a) の生成後書換に事実上限定され、
「構造保証」のはずが**タイミング依存の後付け**になる／要件 5.7 と衝突／実機検証が前提。

### 案 B: 明示維持（`ZOrder::InsertAfter` の配線）

以下の 3 変種があり、**どれを採るかが本 spec 最大の設計判断**である。

- **B1: `WM_ACTIVATE` フックへ相乗り**（brief の想定）
  - 席は空いている（`keyboard.rs:126-131` の早期 return を活性化側へ拡張）。
  - **リスク**: `WM_ACTIVATE` は活性化**処理中**に届く。既定処理による当該窓の前面化と
    順序が競合し、直後に上書きされうる。回避には遅延（`PostMessage` 相当）か
    B2 との併用が要る。→ 設計で要検証。
  - **wintf → areka の import 禁止**により、スコープ解決を wintf 内で行えない。
    汎用コンポーネント（例 `ZOrderPeer { keep_above: Entity }`）を wintf に置き、
    areka が spawn 時に付ける形が最も既存流儀に近い（`BalloonFollow` と同型）。
- **B2: `WM_WINDOWPOSCHANGED` の z-order 変化を見る**
  - `WINDOWPOS.hwndInsertAfter` と `SWP_NOZORDER` の有無で「z-order が動いた」を**確実に**捉える。
    OS 由来の raise（クリック活性化・`SetForegroundWindow` 相当）も、自アプリ由来も、
    **経路を問わず 1 点で捕まる**＝要件 1.4／2.x の網羅性が構造的に上がる。
  - ハンドラは既存（`window_proc/window_pos.rs:36-272`）で、`is_self_initiated()` による
    自己ループ遮断の道具も既にある（`:44`）。**追加の Win32 面がゼロ**。
  - リスク: 再調整が相手窓の `WM_WINDOWPOSCHANGED` を誘発する往復。収束条件の設計が要る
    （「既に直前にいるなら何もしない」の同値ガード＝`GetWindow(GW_HWNDPREV)` 比較）。
- **B3: `WindowPos.zorder` を持続値として持たせる**
  - `WindowPos.zorder` は**誰もリセットしない**（`WM_WINDOWPOSCHANGED` の書戻しは
    position/size のみ・`window_pos.rs:114-117`/`:147-148`）ため、一度入れれば持続する。
  - しかし §1.3 のとおり**ゴースト窓の実運用経路②はこれを読まない**。効かせるには
    `enqueue_window_set_pos`（`window_move.rs:452-544`）の `flags`／`hwnd_insert_after` を
    引数化する改造が要る。改造すれば **areka 由来の全書込（ドラッグ・DPI 再射影・復元・
    リサイズ・`\![move]`・バルーン追従）に自動で乗る**＝要件 2.1〜2.5 を 1 箇所で満たせる。
  - 単独では **OS 由来の raise（クリックで前面化）を捉えられない**＝要件 1.2/1.4/2.7 に穴。

> **現実的な組み合わせ**: 「B3（areka 由来の書込に z-order を同乗）＋ B1 か B2（OS 由来の
> raise を捕捉）」が要件全体を覆う最小構成に見える。B3 単独・B1 単独はいずれも穴が残る。

**Pros（案 B 全体）**: 新規 Win32 概念ゼロ／副作用が読み切れる／ヘッドレスの決定論テストに
載せやすい（判断を純関数へ切り出す先例が `dpi_helpers` にある）／要件 3.4 を
`InsertAfter` の性質で構造的に満たす。
**Cons**: 経路網羅が人手（B2 を採ると大幅に緩和）／`SetWindowPos` の呼出回数増（ドラッグ中の
毎フレーム再指定は同値ガードで抑える設計が要る）。

### 案 C: A＋B 併用

brief の評価（A が効いていれば B は恒真＝空虚な保険／A が壊れたとき B が症状を隠す）は妥当。
ただし **A の実機可否が未知**である以上、**「A を試す → 壊れたら B」という順序**自体は
段階実装として合理的で、これは併用ではなく**分岐**である。

### 案 D（本分析で新出）: owner を張らず、**wintf に「窓ペアの前後関係」コンポーネントを新設**

`ZOrder` は既に `InsertAfter(HWND)` を持つが、**entity 参照ではなく HWND 参照**なので
areka が HWND を握る必要がある。wintf 側に `KeepDirectlyAbove { peer: Entity }` のような
**entity 参照の宣言**を置き、wintf 内の system が
（`WindowHandle` 解決 →`SetWindowPosCommand` 発行）を担えば、
areka は spawn 時に 1 コンポーネント付けるだけで済み、**wintf → areka の import 禁止も守れる**。
B1/B2/B3 のどれを起動条件にしても、この宣言層は共通で使える。
`BalloonFollow`（areka 側の同型宣言）と `DpiSuggestedRectPolicy`（wintf が読む areka 付与の政策）
という**既存の 2 先例**にそのまま乗る形であり、本リポジトリの流儀に最も合致する。

---

## 4. 検証（要件 7）の成立性

### 4.1 決定論テスト（要件 7.1）

- **判断の純関数化**が既に確立した流儀である。先例:
  `dpi_suggested_position_decision`（`window_proc/dpi_helpers.rs`・呼出は `window_pos.rs:372-374`）が
  「政策 → 書く/書かない」を `Option` で返し、ハンドラ側は 1 個の `if let` で分岐する。
  同型で `decide_zorder_fix(activated_kind, scope, pair, current_order) -> Option<ZOrderFix>` を
  切り出せば、World も実 HWND も不要な純関数テストになる。
- **配送テスト**は wintf クレート内に置く（`dispatch_window_message` が `pub(crate)`）。
  `window_proc/mod.rs:92-254` に既存の代表メッセージ配送テスト群があり、そこへ
  `WM_ACTIVATE`（活性化側）を足す形になる。
- **ログの検証**は `crate::ecs::test_support::capture_under_filter` が既にある
  （`lifecycle.rs:255` ほかで使用）。要件 6.1〜6.3 の判定語をこれで固定できる。
- **注意（既知の落とし穴）**: 捕捉ハーネスは `with_default`（スレッドローカル）であり、
  `log` クレート経由の記録は届かない。「無いこと」の主張は wintf 自身の `tracing` 出力に
  限定し、対照ケースを併置する（`lifecycle.rs:228-341` にその作法の完成形がある）。

### 4.2 実機サインオフ（要件 7.2〜7.5）

- 既存の作法: `AREKA_APP_SMOKE_EXIT_MS`（`main.rs:868`）で有界時間の自動終了、
  `RUST_LOG` で水準を上げてログ grep。
- **画素は要らない**。GPU 合成窓はスクリーンショットが効かないが、**z-order は OS に問い合わせられる**:
  - `GetWindow(char_hwnd, GW_HWNDPREV)` が `balloon_hwnd` と一致するか＝「直前にいる」の**厳密判定**。
  - `GetForegroundWindow()` からの z-order 走査で「ゴースト全窓が前面窓より後ろ」＝要件 4.1/4.2。
  - いずれも `windows` crate で追加依存なしに呼べる。**現状 `GetWindow` は 1 件も無い＝新設**。
- 提案する観測レコード（要件 6.1／7.2 を 1 本で満たす）: 調整の**前後**で
  `scope` / `対象 hwnd` / `insert_after hwnd` / **調整後に実測した `GW_HWNDPREV`** を
  専用 target へ出す。実測値を載せることで「指令は出したが効かなかった」を切り分けられる
  （`placement/diag.rs` の `DIAG_TARGET` 方式が先例）。
- 要件 7.3（拡大率の異なる複数ディスプレイ間の移動）は、既に実機 2 水準サインオフの
  前例がある（`collision-dpi-hittest` の 125%/200%）。同じ手順に z-order 実測行を足すだけで済む。

---

## 5. 規模と Risk

| 案 | 規模 | Risk | 一行根拠 |
|---|---|---|---|
| A（owner） | **M**（3〜7 日） | **High** | 導入自体は小さいが、WUC／クリック透過との共存が未知で実機検証が前提。要件 5.7 との衝突裁定と、破棄経路（`WindowRegistry` drop）の二重 `DestroyWindow` 対処が付随する |
| B1 単独 | S | Medium | 席は空いているが、活性化処理中の順序競合が未検証で、OS 由来 raise を取りこぼす穴が残る |
| B2＋B3 | **M**（3〜7 日） | **Medium** | 追加 Win32 面ゼロ・既存フック 2 点と単一 funnel 1 点の改造。収束条件（同値ガード）と `enqueue_window_set_pos` の引数追加が主作業 |
| D（宣言コンポーネント層）を A/B いずれかと併用 | +S | Low | 既存 2 先例（`BalloonFollow`／`DpiSuggestedRectPolicy`）に完全に乗る |
| 実機観測面（`GetWindow` 走査＋ログ） | S | Low | 追加依存なし・純粋な観測追加 |

**総合**: 「A を最初に実機検証 → 不可なら B2＋B3（宣言層 D 経由）」で **M / Medium**。
A が通れば M / Low へ落ちるが、要件 5.7 の裁定は A を採る場合に**必ず**必要。

---

## 6. Research Needed（design フェーズへ持ち越す不確定）

1. **owner 化と WUC／クリック透過の共存**（実機のみ）。`WS_EX_NOREDIRECTIONBITMAP` 窓に
   `GWLP_HWNDPARENT` を後付けした場合の描画・α ヒットテスト・`WS_EX_TRANSPARENT` トグルの挙動。
2. **`WM_ACTIVATE` 内での `SetWindowPos` が既定の前面化に上書きされないか**（実機／実測）。
   上書きされるなら B2（`WM_WINDOWPOSCHANGED`）へ寄せる根拠になる。
3. **`SetWindowLongPtrW(GWLP_HWNDPARENT)` の可否と副作用**（MSDN 上は top-level 窓の owner 変更に
   使える系だが、`WS_EX_TOOLWINDOW`＋`WS_POPUP`＋GPU 合成窓での実挙動は未検証）。
4. ~~**ukadoc `\v` 項の原文**~~ → **2026-08-11 の要件ディスカッションで解決（クローズ）**。
   ukadoc MCP の `get_doc("ukadoc:list_sakura_script:_5cv:1")` で逐語取得できた（当初の
   検索クエリが索引に当たらなかっただけで、項自体は在る）。原文は
   「手前に表示する。このスクリプト以降、常に最前面に表示されるようになる。ただしスコープごとの重なり(本体側と相方側どちらが上にくるか)はユーザの操作次第。ゴースト終了まで有効。」
   （`https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5cv:1`）。
   **要件 3 の根拠は逐語で成立**し、しかも「`\v` で常時最前面へ上げた状態でさえスコープ間順序は
   利用者操作次第」という強い形であるため、`\v` 未適用の既定状態には当然に及ぶ。
   requirements.md の Introduction へ出典 URL・取得日・原文を明記済み。
   なお `windowstate` 3 種と `OnWindowState*`／`OnFullScreenApp*`
   （Ref0＝`system`／`script`／`sakuraapi`／`user`／`fullscreen`）は MCP で**逐語確認済み**で、
   brief の先送り語彙表と一致する。
5. **ドラッグ中の z-order 再指定頻度**。②に z-order を同乗させると毎ドラッグイベントで
   `SetWindowPos` に z 変更が乗る。同値ガード（`GW_HWNDPREV` 比較 or 直近適用値のキャッシュ）を
   どこに置くか（`clickthrough/controller.rs` の `last_applied` 差分ガードが先例）。
6. **`balloon-visibility`（W6 並走）とのシーム**。要件 2.6 は現状で発火経路が無い。
   vis が「再表示」を実装するときに本 spec のどの関数／コンポーネントを呼ぶか、
   **相互登記の具体形**を design で明文化しておく（roadmap の因果 `zorder→vis`）。

---

## 7. design フェーズへの推奨

- **優先アプローチ**: 「**案 A の実機可否を最初のタスクとして確定 → 不可なら案 B2＋B3**、
  いずれの場合も**案 D の宣言コンポーネント層を経由**する」。D を挟むと、A/B の分岐が
  areka 側の spawn コードへ波及しない（付けるコンポーネントは同じ）。
- **要件 3.4 は手段の選択で構造的に満たせる**: `HWND_TOP`／`ZOrder::Top` を**使わない**こと。
  `InsertAfter` は指定窓 1 個だけを動かすため、他スコープの相対順に触れない。
- **要件 8 は現状すでにほぼ成立**（`WS_EX_TOPMOST` 無し・`ZOrder::TopMost` が語彙として存在）。
  design では「`TopMost` を本 spec で**使わない**」ことを明示的な不変条件として書けばよい。
- **観測は「指令」と「結果」の 2 値で残す**。`SetWindowPos` を呼んだ事実だけでは
  要件 7.2 を満たせない（過去に「指令は出ているが効いていない」型の誤診の前例がある）。
  調整後の `GW_HWNDPREV` 実測値を同じ行に載せること。

---

## 8. 設計判断項目（要件ディスカッションへの申し送り）

1. ~~**案 A（owner）／案 B（明示維持）のどちらを本線とするか**~~ → **2026-08-11 の要件ディスカッションで
   開発者が裁定（クローズ）: 案 A を基本とし、実機で問題が出た場合に案 B へフォールバックする。**
   根拠は「案 A の障害が実際に出るかは実機でしか判らない」。roadmap の W6 編成条件⑶
   （案 A の WUC 共存実機検証が最初のタスク）はそのまま有効。design のタスク分割は
   「⓪案 A の実機可否判定 → 通れば A で完成／通らなければ B2＋B3 へ切替」の分岐構造にすること。
   フォールバック判定の基準（何が壊れたら B へ移るか）を design で明文化すること。
   なお**要件 5.7 を根拠に案 A を却下する読みは棄却された**（下記 #4 参照）。
   また owner 関係がクリック・当たり判定・クリック透過を阻害するという懸念は成立しない
   （入力の無効化はモーダルダイアログが `EnableWindow(FALSE)` で明示的に行うものであり、
   素の owner 関係の効果は「owned が owner より手前」「owner 破棄で owned 破棄」
   「owner 最小化で owned 非表示」の 3 点のみ）。残る実機未知は §6-1／§6-3 のとおり
   WUC 合成・`GWLP_HWNDPARENT` 後付けの側にある。
2. **案 B を採る場合の起動条件**: B1（`WM_ACTIVATE`）／B2（`WM_WINDOWPOSCHANGED` の z 変化）／
   B3（`WindowPos.zorder`＋funnel 改造）の組み合わせ。**B3 単独は OS 由来の raise を、
   B1 単独は areka 由来の再配置を、それぞれ取りこぼす**。要件 1.4「どの操作経路でも破らせない」を
   満たす最小の組は「B3＋(B1 か B2)」に見えるが、B2 単独で足りるかは #6 の検証次第。
3. **`enqueue_window_set_pos`（`window_move.rs:452`）に z-order 引数を足すか否か。**
   足せば areka 由来の全書込に自動で乗る（要件 2.1〜2.5 を 1 箇所で）。足さない場合は
   別経路の `SetWindowPos` を新設することになり、「本経路を迂回する第二の書込経路を作らない」
   という同関数の既存規約（doc `:410`）と衝突する。
4. ~~**要件 5.7（破棄の巻き込み禁止）と案 A の owner 破棄カスケードの裁定。**~~
   → **2026-08-11 の要件ディスカッションで解決（クローズ）: 要件 5.7 の文面を改訂した。**
   5.7 は「他スコープの窓を巻き込まない」に限定し、**5.8 で同一スコープのペア同時消滅を明示的に許容**
   （両窓は対で生成され対で破棄される 1 単位。現物の `GhostWindowMarker` の `on_remove` hook
   〔`spawn.rs:122-142`〕が既にペア同時消滅を実装しており、旧文面のままでは既存実装が
   要件違反になっていた）。よって **owner 破棄カスケードは要件違反ではなく、案 A は生存**。
   **残る design 課題**: `WindowRegistry` drop による二重 `DestroyWindow`
   （`lifecycle.rs:97-116`）——新設した要件 5.9「破棄処理が重複しても異常終了しない」を
   どう満たすか（案 A を採る場合は必須）。
5. **z-order 政策の所有層**: wintf に汎用コンポーネント（案 D）を置くか、areka 側で完結させるか。
   前者は `DpiSuggestedRectPolicy`（areka が付け wintf が読む）と `BalloonFollow`（areka 内の
   entity 参照宣言）の**両方の先例に合致**するが、wintf の公開面が増える。
6. **要件 1.3（バルーン活性化時にキャラを引き上げる）の具体形**。
   ⑴キャラをバルーンの直後へ入れる（2 窓ペアが 1 単位で浮上）か、
   ⑵バルーンだけ前に出て、キャラは元の位置に留まるか。brief の Open Question 2 が未決で、
   要件 1.3 は⑴を採った表現になっている。**⑴を採ると「バルーンを掴むとキャラも前に出る」＝
   ペア浮上**であり、これは他アプリの窓 2 枚ぶんの前後関係を動かす。意図どおりか確認が要る。
7. **ドラッグ中の再指定頻度と同値ガードの置き場所**（Research #5）。
   毎フレーム z-order を指定すると、他アプリ窓との相対順が毎フレーム動くことになる。
   「既に直前にいるなら何もしない」の判定を何で行うか（`GW_HWNDPREV` 実測 vs. 適用値キャッシュ）。
8. **実機サインオフの判定手段として `GetWindow` 走査を採用するか。**
   採用すれば要件 7.2〜7.5 が画素なしで機械判定でき、要件 6.1 の診断ログと同じ行で
   「指令」と「結果」を突合できる。採用しない場合、要件 7.2 の「判定できる証跡」を
   何で構成するかを別途決める必要がある（GPU 合成窓はスクリーンショット不可）。
9. **要件 2.6（非表示→表示）の履行時期**。現状 hide/show の実装は areka に 1 件も無く、
   本 spec 単独では**発火経路が無い＝空虚な保証**になる。
   `balloon-visibility`（W6 並走）が呼ぶシームとして定義し、
   相互登記を design の Boundary へ明記するのが妥当か。
10. **要件 4.4（背面でも相対順を保持）の検証方法**。
    他アプリ活性化時にゴースト一式が沈む間、バルーン⇄キャラの相対順が保たれることを
    どう観測するか（前面から外れた後の z-order 走査で判定可能だが、観測タイミングの決め方が要る）。
11. **要件 4.1「背面に置く」の能動／受動の別**（2026-08-11 の要件ディスカッションで追加）。
    要件 4.1 は「ゴーストの全窓を活性化された窓より背面に置く」と動作動詞で書かれているが、
    現物では `WS_EX_TOPMOST` 無し（`spawn.rs:332-337`）ゆえ **OS 既定で勝手に沈む**のであり、
    areka が能動的に沈める処理は不要である。design では 4.1 を
    ⑴「沈んだ状態を妨げない＝維持コードが持ち上げ返さない」という**反証責務**として実装する
    （§2 の 🟡 判定はこの読み）か、⑵能動的な背面化処理を持つか、を明示的に決めること。
    ⑵を採ると要件 4.3（常時最前面にしない）・要件 3（スコープ間非強制）と干渉しうる。
    要件文はどちらの読みでも満たせるため、**要件は改訂せず design で手段を確定する**。
