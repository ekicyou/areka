# W7a-V: wintf ウィンドウ・メッセージ × 脆弱性レビューと非破壊対策

- status: completed
- commit: （親が S10 準拠でコミット）

## 観点・基準・範囲

- セルID: W7a-V（領域 W7a「wintf ウィンドウ・メッセージ」 × 観点 V「脆弱性レビュー」）。性質: **非挙動変更**（脆弱性点検＋挙動非破壊な対策のみ）。Feature Flag Protocol 不要。
- requirements（source 番号）: 2.3（脆弱性レビュー＋挙動非破壊対策）・2.4（挙動変更を伴う対策→提案記録）・2.5（前後 S2 非破壊）・2.7（列順 T→S→V。W7a-T1/T2/S 完了済みの回帰検知器上で実行）・2.8（テスト保護外でも深く解析・安全適用不能は提案記録）・4.1（自己レビュー＋検証）・5.1（外部観測可能挙動を変更しない）・5.2（挙動変更必要時は提案記録）。
- design: Security Considerations（L512-516: unsafe 境界・ポインタ有効性/ライフタイム/Send-Sync 妥当性・整数変換の切り捨て/オーバーフロー・Win32/COM ハンドルのリーク/二重解放・外部入力検証・panic 経路 DoS を点検し、挙動を変えない範囲＝内部チェック・debug_assert・安全な型置換・SAFETY 注記のみ投入。API/エラー応答を変える対策は proposals へ）、CellExecutor 観点別規則 V（L338）、提案記録様式（L453）、セル断片様式（L440）。
- 領域（boundary = `crates/wintf/src/ecs/window/` + `ecs/window_proc/`、tests/ の該当ドメイン含む）: window/（mod.rs・components.rs・dpi.rs・window_pos.rs・command.rs・window_handle.rs・window_system.rs・monitor.rs）＋ window_proc/（mod.rs・lifecycle.rs・window_pos.rs・mouse_move.rs・mouse_click.rs・mouse_dblclick_wheel.rs・keyboard.rs・dpi_helpers.rs）の計16ファイル。境界外には一切触れていない。
- 起点: W7a-S 適用後のクリーンなワークツリー（親検証済みベースライン 1625 passed / 0 failed）。
- **本領域の最重要点検対象**: Win32/HWND メッセージ依存・unsafe（WndProc）密集域。HWND ライフサイクル（生成⇔破棄・解放後使用・リーク）と、HWND を保持する型の手動 `unsafe impl Send/Sync` の妥当性（W5a-V/W5b-V の自動付与非対称性知見の適用）を重点追跡した。

## 点検手法

境界内16ファイル（うち in-source tests: dpi.rs/window_pos.rs/command.rs/components.rs/monitor.rs/dpi_helpers.rs/mouse_click.rs/mouse_move.rs）を grep（`unsafe`/`unwrap(`/`expect(`/`panic!`/`unreachable!`/`todo!`/`as i16|i32|u32|u16|usize|isize|u64`/`[添字]`/`HWND`/`DestroyWindow`/`thread_local`/`OnceLock`/`static`）＋全文精読で走査。unsafe 境界（手動 Send/Sync 5型・WndProc 内 FFI・生ポインタ deref）・HWND ライフサイクル（WM_NCCREATE↔WM_NCDESTROY、WM_CLOSE/DestroyWindow、on_window_handle_add/remove）・整数変換（WPARAM/LPARAM 抽出）・マルチスレッド境界（WndProc スレッドと ECS スレッド、thread_local、OnceLock、AtomicI32 echo カウンタ）の4観点を端から端まで追跡した。

unsafe impl Send/Sync の自動付与有無は **windows-rs 0.62.2 のソースを直接 grep して裏取り**した（wintf は workspace `windows = "0.62.2"` を使用。`crates/wintf/Cargo.toml` + ルート Cargo.toml `[workspace.dependencies.windows]` で確認）:
- `HWND`: `pub struct HWND(pub *mut core::ffi::c_void)`（Foundation/mod.rs:5670）。`impl Send/Sync for HWND` は **0 件**（未生成）。
- `HINSTANCE`: `pub struct HINSTANCE(pub *mut core::ffi::c_void)`（同:5458）。`impl Send/Sync for HINSTANCE` は **0 件**（未生成）。

これにより「HWND/HINSTANCE は `*mut c_void` newtype で windows-rs が Send/Sync を自動付与しない」＝本領域の HWND 保持型の手動 `unsafe impl` は**冗長ではなく必須**であることを実証した（W5a-V の `IDWriteTextLayout`＝自動付与あり・冗長、とは逆のケース。むしろ W5b-V の WIC 型＝自動付与ゼロ・必須、と同方向）。

## 発見した脆弱性候補と判定

### 1. unsafe 境界（手動 `unsafe impl Send/Sync`）— HWND 保持4型は健全かつ必須。SAFETY 注記を crate 標準へ格上げ（適用）

境界内の手動 `unsafe impl Send/Sync` は **5型**: `Window`（components.rs:151-152、`parent: Option<HWND>` 保持）・`WindowHandle`（window_handle.rs:35-36、`hwnd: HWND` + `instance: HINSTANCE`）・`ZOrder`（window_pos.rs:49-50、`InsertAfter(HWND)`）・`WindowPos`（window_pos.rs:109-110、`zorder: ZOrder` 経由で HWND）・`SendWeak`（window_proc/mod.rs:35-36、`Weak<RefCell<EcsWorld>>`）。点検の結果すべて健全:

- **HWND 保持4型（Window / WindowHandle / ZOrder / WindowPos）は健全かつ必須**: 上記 windows-rs ソース実証のとおり HWND/HINSTANCE は自動 Send/Sync を持たないため、これらを内包する4型も自動導出できず、手動 impl は load-bearing。健全性: HWND/HINSTANCE は不透明な OS ハンドル（実質ウィンドウ/インスタンスを指す整数値）で、スレッド間で**値として**受け渡しても所有権・解放責務を伴わない（ウィンドウ破棄は WM_NCDESTROY/DestroyWindow が担い、これら ECS コンポーネントの Drop ではない）。4型はいずれも ECS コンポーネント（Bevy は Send+Sync を要求）として**メインスレッドのシステム・ライフサイクルフックからのみ**参照・更新される。**従来これら4型には SAFETY 注記が無かった**（5型中 `SendWeak` のみ2行コメントあり）ため、crate 標準の HWND 取り扱い方針である `drag/context.rs:26-30`（`WindowDragContext` の HWND 健全性注記）に揃えて根拠を明文化する SAFETY 注記へ格上げした（適用1〜4）。
- **`SendWeak`（`Weak<RefCell<EcsWorld>>`）は健全**: `Weak<RefCell<_>>` は `RefCell` が `!Sync`・`Rc`/`Weak` が `!Send + !Sync` のため自動 Send/Sync を持たず、手動 impl は `static ECS_WORLD: OnceLock<SendWeak>`（`OnceLock<T>` の `T: Send + Sync` 境界）を満たすために必須。健全性は「アクセスは常に単一スレッド（メインスレッド）」の不変条件に依拠する: `set_ecs_world` は WinThreadMgr 初期化時にメインスレッドから1回 set、`try_get_ecs_world().upgrade()` で得た `Rc` を実際に借用（`borrow`/`borrow_mut`）するのは `ecs_wndproc` 経由の各ハンドラのみで、WndProc はウィンドウを作成したメインスレッドからのみ呼ばれる。よって RefCell 借用規則も Rc 参照カウントも単一スレッド上でのみ操作されデータ競合なし。OnceLock は弱参照ポインタの move/共有のみを担う。従来の2行コメント（「EcsWorld はメインスレッドでのみアクセス／wndproc もメインスレッドから呼ばれるため安全」）は正確だが根拠が簡潔だったため、`Weak<RefCell<>>` が非 Send/Sync である理由・OnceLock 境界を満たす目的・単一スレッド不変条件の具体経路を明示する SAFETY 注記へ拡充した（適用5）。
- **判定**: 5型すべて健全。HWND 保持4型は**必須**（撤去すればコンパイル不能）、SendWeak も**必須**。冗長な型は本境界に存在しない（W5a-V の `IDWriteTextLayout`／W5b-V の `ID2D1Bitmap1` のような自動付与済み冗長型はゼロ）。**挙動非破壊対策として SAFETY 注記5箇所で根拠を明文化**（コメントのみ・コード挙動不変）。**proposals 不要**（撤去不可＝必須のため、設計変更でもない）。

### 2. WndProc 内 FFI 呼び出し・Win32 メッセージ生ポインタ deref — 現状安全（対策不要）

WndProc ハンドラ内の `unsafe` ブロックを個別判定:

- **メッセージ LPARAM の生ポインタ deref（null ガード済み）— 安全**: (a) `WM_NCCREATE`（lifecycle.rs:23-25）`lparam.0 as *const CREATESTRUCTW` → `if !cs.is_null()` → `unsafe { (*cs).lpCreateParams as isize }`、(b) `WM_WINDOWPOSCHANGED`（window_proc/window_pos.rs:50-52）`lparam.0 as *const WINDOWPOS` → `if !windowpos.is_null()` → `unsafe { &*windowpos }`、(c) `WM_DPICHANGED`（同:287-289）`lparam.0 as *const RECT` → `if !suggested_rect_ptr.is_null()` → `unsafe { *suggested_rect_ptr }`（RECT を値コピー）。いずれも **Win32 が当該メッセージで保証する有効ポインタ**を null ガード後に読み取る標準パターンで、参照を構造体外へ漏らさない（即座に値を取り出す/スコープ内で借用）。OS が渡すポインタの有効性は Win32 契約（メッセージ処理中は有効）に依拠し、これは実 WndProc 経路でのみ成立する前提のため新たな明文化を要する非自明な不変条件はない。**現状安全。**
- **標準 windows-rs FFI 呼び出し — 安全**: `DefWindowProcW`（mod.rs:82）・`GetWindowLongPtrW`/`SetWindowLongPtrW`（mod.rs:89・lifecycle.rs:27/54）・`DestroyWindow`（lifecycle.rs:127）・`BeginPaint`/`EndPaint`（lifecycle.rs:109-111）・`ScreenToClient`/`GetClientRect`（mouse_move.rs:32-46）・`TrackMouseEvent`（mouse_move.rs:139）・`guarded_set_window_pos`（mouse_move.rs:409・window_proc/window_pos.rs:351、内部で `SetWindowPos`）。引数はスタック上の有効値（`POINT`/`RECT`/`PAINTSTRUCT` を `&mut` で渡す）・メッセージ由来 HWND で、戻り値は `let _`/`Result` で受ける。新たに明文化を要する非自明な不変条件なし。**現状安全（対策不要）。**

### 3. HWND ライフサイクル（生成/破棄・解放後使用・リーク）— 現状安全（対策不要）

HWND の生成⇔破棄サイクルと HWND↔Entity マッピングの破棄整合性を端から端まで追跡し、**対称かつ use-after-free/リーク無し**と判定:

- **生成**: `create_windows`（window_system.rs、排他システム）→ `CreateWindowExW`（CREATESTRUCT.lpCreateParams に Entity ビット）→ **WM_NCCREATE** が `entity_bits` を `GWLP_USERDATA` に保存（lifecycle.rs:23-28、`ID 0 も有効` コメントどおり Entity::PLACEHOLDER でない限り保存）→ WindowHandle コンポーネント挿入 → `on_window_handle_add`（window_handle.rs）が App へ通知。
- **破棄経路A（despawn 起点）**: Entity despawn → `on_window_handle_remove` フック（window_handle.rs:250-274）が `PostMessageW(hwnd, WM_CLOSE)` を投函 → **WM_CLOSE**（lifecycle.rs:121-129）が `DestroyWindow(hwnd)` → **WM_NCDESTROY**（lifecycle.rs:36-57）が `get_entity_from_hwnd` → `despawn(entity)`（冪等・既に despawn 済みでも安全）＋ `SetWindowLongPtrW(GWLP_USERDATA, 0)` で USERDATA クリア。
- **破棄経路B（ウィンドウ起点、ユーザのクローズ等）**: WM_CLOSE → DestroyWindow → WM_NCDESTROY → despawn + USERDATA クリア。despawn が再度 `on_window_handle_remove` を発火し `PostMessageW(WM_CLOSE)` を破棄中の HWND へ投函するが、これは Win32 仕様で no-op（無効 HWND への PostMessage はエラー戻り、`let _ =` で吸収）。
- **解放後使用なし**: USERDATA は **despawn の後**にクリアされ（lifecycle.rs:49→54）、以後 `get_entity_from_hwnd` は `GetWindowLongPtrW` が 0 を返し `Entity::try_from_bits(0)` で None に縮退する（破棄後の stale Entity ビット参照を構造的に排除）。Entity を保持する HWND が破棄された後にハンドラが当該 HWND でメッセージを受けても、USERDATA=0 → None → `DefWindowProcW` 委譲で安全。
- **リークなし**: 各 HWND は破棄経路で必ず `DestroyWindow` に至る（WM_CLOSE が唯一の入口で必ず DestroyWindow を呼ぶ）。`DestroyWindow` 漏れの経路は本境界になし。WindowHandle/HINSTANCE は OS が所有するハンドルで、本コンポーネントの Drop が解放するのではなく WM_NCDESTROY が破棄を担う（二重解放なし）。
- **判定**: 生成⇔破棄は対称、USERDATA クリアにより use-after-free なし、DestroyWindow 漏れなし。**現状安全（対策不要）。** 実起動 S7（areka 起動→初期化→終了、パニック・エラーログなし）が HWND ライフサイクル全体の最終回帰検知器。

### 4. メッセージパラメータの整数変換（WPARAM/LPARAM の as キャスト）— 現状安全（既知 P64 参照）

WPARAM/LPARAM からの `as` キャストを全数判定。**いずれも `& 0xFFFF` でマスクしてから幅縮小**するため切り捨て/符号反転のサプライズなし:

- **LPARAM 座標抽出**: `(lparam.0 & 0xFFFF) as i16 as i32` / `((lparam.0 >> 16) & 0xFFFF) as i16 as i32`（mouse_move.rs:28-29/110-111・mouse_click.rs:33-34・mouse_dblclick_wheel.rs:40-41）。下位/上位16bit を抽出後 `as i16`（符号付き座標、負座標を正しく復元）→ `as i32` 拡大（無損失）。マスク済みのため上位ビット混入なし。W7a-T1/T2 が `DPI::from_WM_DPICHANGED` 等で同型のビット抽出を特性化済み。
- **WPARAM 抽出**: 修飾キー `(wparam_val & 0x04)!=0`/`(wparam_val & 0x08)!=0`、XBUTTON `((wparam.0 >> 16) & 0xFFFF) as u16`、wheel delta `((wparam.0 >> 16) & 0xFFFF) as i16`（符号付き）、activation_state `(wparam.0 & 0xFFFF) as u32`（keyboard.rs:121）。いずれもマスク後キャストで切り捨て非発生。VK_ESCAPE 比較 `wparam.0 == VK_ESCAPE.0 as usize`（keyboard.rs:23）は u16→usize 拡大で無損失。
- **ハンドル/Entity 変換**: `get_entity_from_hwnd` の `entity_bits as u64`（mod.rs:90、`isize`→`u64`）は `Entity::try_from_bits` が妥当性検証（不正ビット→None）するため安全。`hwnd.0 as usize`（ログ整形）・`hmonitor.0 as isize`（monitor.rs:132）はハンドル値のフォーマット/格納で観測影響なし。
- **判定**: マスク済み抽出のため整数変換に起因する切り捨て/符号反転/オーバーフローはなし。**現状安全。** これら抽出ロジックがハンドラ本体にインライン埋め込みで単体到達不能な点は **既知 P64**（W7a-T2 記録・純粋ヘルパ抽出候補）に該当し、本 V セルでは新規採番せず参照に留める（抽出は挙動非破壊な構造変更だが S 観点／判断に迷う構造変更であり V 観点の脆弱性ではない）。

### 5. マルチスレッド境界（WndProc スレッドと ECS スレッド・thread_local・echo カウンタ）— 現状安全（対策不要）

- **World 共有（`ECS_WORLD: OnceLock<SendWeak>`）**: 所見1で判定。set はメインスレッド1回・borrow は WndProc（=メインスレッド）のみ。`try_borrow_mut`/`try_borrow` で再入時はスキップ（WM_WINDOWPOSCHANGED window_proc/window_pos.rs:49、WM_DPICHANGED 同:312 等）し RefCell の二重借用 panic を回避。単一スレッド前提のため Send/Sync 跨ぎの実共有は発生しない。**安全。**
- **echo カウンタ（`SELF_INITIATED_DEPTH: AtomicI32`、command.rs:33）**: `guarded_set_window_pos` が RAII `SetWindowPosGuard` で +1/−1（Drop 保証・正常/`?`/panic いずれでも復元）。`SetWindowPos` → `WM_WINDOWPOSCHANGED` は**同一スレッド同期発火**（command.rs:30 コメント）のため、ハンドラ内 `is_self_initiated()` が echo を正しく判定できる。`AtomicI32`+`Relaxed` は型としてはスレッド安全だが実運用は単一スレッド（UI スレッド）。同期発火前提が崩れない限り echo 判定は健全。**安全。**
- **thread_local キュー/コンテキスト**: `WINDOW_POS_COMMANDS`（command.rs:126、SetWindowPos 遅延キュー）・`DPI_CHANGE_CONTEXT`（components.rs:43、DPI 変更中心保持補正信号）はいずれも UI スレッドの thread_local で、enqueue/flush・set/take が同一スレッドで完結（drag thread_local も同様、keyboard/mouse ハンドラ内アクセスは WndProc=UI スレッド）。スレッド跨ぎ共有なし。`DpiChangeContext::take()` は消費的取得（2回目 None）で WM_DPICHANGED→WM_WINDOWPOSCHANGED の1回限り受け渡しを保証。**安全。**
- **判定**: WndProc/ECS は同一 UI スレッドで動作し、共有状態（OnceLock World 弱参照・Atomic echo カウンタ・各 thread_local）はいずれも単一スレッド前提が一貫して守られ、RefCell 再入は try_borrow でガード。データ競合経路なし。**現状安全（対策不要）。**

### 6. panic 経路 — 現状安全（対策不要）

境界内プロダクション経路の `unwrap()`/`expect()`/`panic!`/`unreachable!`/`todo!`/生添字 `[i]` は **ゼロ**（grep + 精読確認）。grep ヒットはすべて `#[cfg(test)]` 内（components.rs:330 の `mod tests`、dpi_helpers.rs:199/215/241、mouse_click.rs:477/496/543 の各テスト）。全ハンドラは `HandlerResult = Option<LRESULT>` を返し、Entity/World 取得失敗や境界外座標は `None`→`DefWindowProcW` 委譲で縮退する設計（`let...else { return None }` イディオム）。外部入力由来の DoS panic 経路なし。**現状安全（対策不要）。**

## 適用した挙動非破壊対策（4 ファイル・8 箇所、+84/−2 行）

| ファイル | 箇所 | 対策 | 種別 | 根拠 |
|----------|------|------|------|------|
| `window/components.rs` | `Window` の `unsafe impl Send/Sync` 直前（:151 前） | `Window.parent: Option<HWND>`（HWND は `*mut c_void` newtype・windows-rs 0.62.2 で非 Send/Sync）により手動 impl が**必須**である旨と健全性根拠（HWND は値渡しの不透明識別子・メインスレッドのみ参照）を記す SAFETY 注記（9 行） | SAFETY/不変条件コメント | コメントのみ・コード挙動不変。crate 標準（`drag/context.rs` の WindowDragContext）の HWND 健全性注記へ格上げ。必須性は windows-rs ソース実証済み。 |
| `window/window_handle.rs` | `WindowHandle` の `unsafe impl Send/Sync` 直前（:35 前） | `hwnd: HWND`/`instance: HINSTANCE`（両者 `*mut c_void` newtype・非 Send/Sync）により**必須**である旨＋健全性根拠（OS ハンドル・破棄は WM_NCDESTROY が担い本構造体 Drop ではない・メインスレッドのみ参照）を記す SAFETY 注記（9 行） | SAFETY/不変条件コメント | コメントのみ・コード挙動不変。 |
| `window/window_pos.rs` | `ZOrder` の `unsafe impl Send/Sync` 直前（:49 前） | `ZOrder::InsertAfter(HWND)` により**必須**である旨＋健全性根拠を記す SAFETY 注記（8 行） | SAFETY/不変条件コメント | コメントのみ・コード挙動不変。 |
| `window/window_pos.rs` | `WindowPos` の `unsafe impl Send/Sync` 直前（:109 前） | `WindowPos.zorder: ZOrder` 経由で HWND を内包するため**必須**である旨＋健全性根拠（残フィールドはプレーンデータ）を記す SAFETY 注記（8 行） | SAFETY/不変条件コメント | コメントのみ・コード挙動不変。 |
| `window_proc/mod.rs` | `SendWeak` の `unsafe impl Send/Sync` 直前（:35 前、旧2行コメント置換） | `Weak<RefCell<EcsWorld>>` が非 Send/Sync である理由・`OnceLock<T: Send+Sync>` 境界を満たす目的・単一スレッド（メインスレッド）不変条件の具体経路（set 1回／borrow は WndProc のみ）を明示する SAFETY 注記（13 行・旧2行置換） | SAFETY/不変条件コメント | コメントのみ・コード挙動不変。従来の正確だが簡潔な注記を crate 標準根拠ブロックへ拡充。 |
| `window/window_pos.rs` | in-source `mod tests`（`use super::*;` 直後） | `test_window_pos_types_are_send_sync`（`fn assert_send_sync<T: Send+Sync>()` を `ZOrder`/`WindowPos` に適用するコンパイル時静的表明、コメント込み） | 特性化/回帰テスト（S9 命名準拠） | `ZOrder`/`WindowPos` の Send+Sync 不変条件をコンパイル時に固定（device 非依存・実 HWND 不要）。将来フィールド追加＋手動 impl 撤去で Send 性が壊れた場合に検出。 |
| `window/components.rs` | in-source `mod tests`（`use super::*;` 直後） | `test_window_is_send_sync`（同上を `Window` に適用） | 特性化/回帰テスト（S9 命名準拠） | `Window` の Send+Sync 不変条件をコンパイル時に固定。 |
| `window/window_handle.rs` | 新規 in-source `mod tests`（ファイル末尾） | `test_window_handle_is_send_sync`（同上を `WindowHandle` に適用） | 特性化/回帰テスト（S9 命名準拠） | `WindowHandle` の Send+Sync 不変条件をコンパイル時に固定。window_handle.rs は従来 in-source テストゼロのため `mod tests` を新規作成（最も安全性関連の高い2ハンドル保持型ゆえ静的表明を追加）。 |

合計 **+84/−2 行**（`git diff --numstat`: components.rs +20/−0・window_handle.rs +24/−0・window_pos.rs +28/−0・window_proc/mod.rs +12/−2）。プロダクションロジックの変更は**ゼロ**（SAFETY コメント5箇所 + コンパイル時静的表明テスト3件のみ。−2 行は SendWeak の旧2行コメントが新 SAFETY 注記へ置換されたもので、コードトークンの削除ではない）。境界内（window/ + window_proc/）に収束。新規テストファイルなし（window_pos.rs/components.rs は既存 `mod tests` へ追記、window_handle.rs は `mod tests` 新規作成。いずれも in-source・統合 tests/ への追加変更なし）。

### 追加した特性化テスト一覧（in-source 3 件）

- `window/window_pos.rs::test_window_pos_types_are_send_sync` — `ZOrder`・`WindowPos` が Send+Sync であることをコンパイル時静的表明で固定（HWND newtype 内包型の不変条件回帰検知）。
- `window/components.rs::test_window_is_send_sync` — `Window` の Send+Sync を固定。
- `window/window_handle.rs::test_window_handle_is_send_sync` — `WindowHandle` の Send+Sync を固定（HWND+HINSTANCE 2ハンドル保持型）。

なお `SendWeak`（window_proc/mod.rs）は private 型（`pub` でない）かつ既に `OnceLock<SendWeak>` の `T: Send+Sync` 境界をコンパイラが強制している（破れば mod.rs 自体がコンパイル不能）ため、別途の静的表明テストは冗長と判断し追加せず（SAFETY 注記の拡充のみ）。HWND 保持4型は ECS コンポーネント登録で Send+Sync が要求されるが、登録箇所での失敗は遠隔のため、各型の定義近傍に局所的な回帰検知器として静的表明を置いた（W5a-V の `test_typewriter_layout_cache_is_send_sync` と同方針）。

## proposals.md へ回した候補

- **新規記録なし**（P66 採番なし）。挙動変更を要する脆弱性対策（panic→Result 化・入力検証の厳格化・整数変換の堅牢化・HWND ライフサイクルのロジック変更・unsafe 設計変更等）に該当する**実在脆弱性は本境界に検出されなかった**。手動 `unsafe impl Send/Sync` は5型すべて**必須**（撤去不可）かつ健全のため、撤去候補としての proposals 化も不要（W5a-V/W5b-V のような自動付与済み冗長型がゼロ）。

既知 proposals の再発見（重複記録なし・参照に留めた）:
- **P64**（W7a-T2）: window_proc メッセージパラメータ抽出ロジックの純粋ヘルパ抽出（インライン埋め込み・3ファイル複製）。所見4で整数変換を再点検したが、抽出は挙動非破壊な構造変更（S 観点／判断に迷う構造変更）であり V 観点の脆弱性ではないため二重記録せず参照に留めた。W7a-S が抽出可否を慎重検討のうえ P64 維持・見送りと結論済み。
- **P63**（W7a-T1）: SetWindowPosCommand キューのテスト用検査 API 欠如。マルチスレッド境界（thread_local キュー）の点検で再確認したが、これはテスト容易化の課題であり脆弱性ではないため参照に留めた。
- **P65**（W7a-S）: create_windows の CompositionMode→ex_style 分岐の純粋関数抽出。S 観点の構造変更候補で V 観点対象外。参照に留めた。

## verification (S2)

- BEFORE: 親検証済みベースライン（W7a-S 直後 = **1625 passed / 0 failed**、クリーンワークツリー）を信頼し省略（design フェーズ0 規定 + 親指示「BEFORE S2 は省略可」に従う）。
- AFTER（必須・全量実施）:
  - `cargo build --workspace` → **成功**（exit 0、wintf/areka 再コンパイル、8.58s）。
  - `cargo test --workspace` → **1628 passed / 0 failed**（ignored 32。全20本の `test result:` 行を awk 合算で実測。`error[`/`^error`/`panicked`/`FAILED` 行ゼロ）。ベースライン 1625 から **+3 = 追加した静的表明テスト3件と一致**（既存テストの削除・変更ゼロ）。
  - 反復検証: `cargo test -p wintf --lib window::` で **46 passed / 0 failed**（W7a-T1/S の 43 + 新規3）。`--lib window_proc::` で **23 passed / 0 failed**（W7a-T2 の 23・本セル追加なし）。
  - 追加3件は初回実行で合格（静的表明＝GREEN by construction = コンパイルが通れば成立）。SAFETY コメント追加はリリース/デバッグ挙動を変えず既存1625件がそのまま通過＝**挙動非破壊**を実証。
- 件数整合（1628 = 1625 + 3）で SAFETY 注記5箇所＋静的表明3件の挙動非破壊を実証。git diff 実測（新規 `#[test]` = 3、削除 0、プロダクションロジック変更 0）と完全一致。

## RED フェーズ代替の検証

追加3件はいずれも既存型の Send+Sync 性の characterization のため RED は N/A（GREEN by construction = コンパイルが通れば成立）。期待値は実装と独立に「内包フィールドがすべて Send+Sync ⇒ 構造体は自動 Send+Sync、ただし HWND/HINSTANCE は非 Send/Sync ゆえ手動 impl が成立を担保する」という型システム規則 + windows-rs 0.62.2 ソース実証（HWND/HINSTANCE が `*mut c_void` で Send/Sync 未生成）から導出した。3件とも初回コンパイルで成立し、矛盾なし。

## clippy（S3・記録のみ・非ブロッカー）

- `cargo clippy -p wintf --lib --message-format=short` の boundary（window/ + window_proc/）span を抽出。**本セルの編集（SAFETY コメント5箇所・静的表明テスト3件）は新規 clippy 警告/error を一切導入していない**。
  - boundary 内 span は **42 件**で、すべて**プロダクションコードの既存 lint**（collapsible_if 多数・`let...else`→`?`（question_mark）8・`drop_non_drop`（mouse_move.rs:146/319/377、借用解放のため load-bearing）3・type_complexity（window_system.rs:24）1・default-unit-struct（window_system.rs:146）1）。これらは W7a-S が R5.5/churn 回避で意図的に据え置いた集合と一致。
  - `window_pos.rs:435` の collapsible_if は W7a-S 記録の `:425`（`SetWindowParentToLayoutRoot::apply`）が本セルの SAFETY コメント追加で +10 行シフトしたもの（同一プロダクションコード・本セル未編集箇所）。追加した `mod tests`（window_pos.rs:455 以降・components.rs:243 以降・window_handle.rs 末尾）の行を指す clippy 診断は**ゼロ**。
- **error 20件はすべて `com/d2d/command_sink.rs`**（`clippy::not_unsafe_ptr_arg_deref`= COM vtable コールバックの生ポインタ引数）であり、**boundary 外**・本セル以前から存在（W7a-T1/T2/S 所見と一致）。boundary 内に error ゼロを実測確認。S3 規定により記録のみ・非ブロッカー。

## flaky

- 既知フレーキー `cue_performance_test::bench_pop_ready_empty_queue`（W7a 境界外 `tests/ecs`）: `cargo test --workspace` の全量実行で `tests/ecs` バイナリは failed=0 で合格（隔離再実行不要）。本セルの変更（SAFETY コメント・静的表明テスト）は cue キュー timing と無関係。

## 自己レビュー

- 実装は本物（モック/スタブ/プレースホルダ/TODO なし）。本セルの変更は SAFETY 注記5・コンパイル時静的表明テスト3のみで、新たな unsafe・スタブを導入していない。プロダクションロジック変更ゼロ。
- 点検は境界内16ファイルを grep＋精読で網羅。unsafe 境界（手動 Send/Sync 5型・WndProc FFI・メッセージ生ポインタ deref）・HWND ライフサイクル（生成⇔破棄対称性・USERDATA クリアによる use-after-free 排除・DestroyWindow リーク無し）・整数変換（マスク済み WPARAM/LPARAM 抽出）・マルチスレッド境界（OnceLock World 弱参照・AtomicI32 echo カウンタ・各 thread_local の単一スレッド前提）・panic 経路の5観点すべてを判定。**unsafe impl Send/Sync の自動付与有無は windows-rs 0.62.2 ソースを直接 grep して裏取り**（HWND/HINSTANCE は `*mut c_void` で Send/Sync 未生成＝手動 impl 必須）し、HWND ライフサイクル・スレッド境界の本番挙動主張を実コードで実証した（過去セルの未確認事実主張による REJECTED を回避）。
- warranted な挙動非破壊対策は (a) HWND 保持4型 + SendWeak の SAFETY 注記 crate 標準化（必須性を windows-rs ソースで実証）と (b) HWND newtype 内包3型の Send+Sync 静的表明テストに限られた。挙動変更を要する実在脆弱性は不検出のため proposals 新規ゼロ。既知 P63/P64/P65 は参照に留めた。
- 件数の実測整合: S2 全量 1628 = 1625 + 3（追加テスト3）。lib window:: 43→46、window_proc:: 23（不変）。追加 `#[test]` git diff 実測 = 3、削除 0、プロダクションロジック変更 0。clippy boundary 42（すべて既存・新規ゼロ）。すべて git diff・cargo test 実測と一致（推測なし）。
- 境界遵守: 変更は `window/{components,window_handle,window_pos}.rs`・`window_proc/mod.rs`（すべて W7a 境界内）のみ。tasks.md 未更新・コミット未作成・境界外/`vendors/`/機能spec文書/proposals.md への変更なし。シェル出力を OS 一時パスへリダイレクトせず、リポジトリルートにスクラッチを残していない（`git status` で確認済み）。
- 非ブロッキング懸念（CONCERNS）: `ecs/graphics/command_list.rs:29` の SAFETY コメントは「windows-rs の COM スマートポインタは自動では Send/Sync にならない」と blanket 主張するが、これは型依存で普遍的に正しくない（D2D/DWrite 型は 0.62.2 で付与済み・W5a-V 所見と一致）。本コメントは **W7a 境界外**のため是正しない（W5a-V が既に CONCERNS 記録済みの重複所見・参照に留める）。
- 結論: 本境界は Win32/HWND/unsafe 密集域ながら脆弱性耐性が高い。最重要の手動 `unsafe impl Send/Sync` は5型すべて**健全かつ必須**（HWND/HINSTANCE が windows-rs で非 Send/Sync である実証に基づく）で、crate 標準 SAFETY 注記＋静的表明テストで根拠を固定した。HWND ライフサイクルは生成⇔破棄対称・USERDATA クリアで use-after-free 排除・DestroyWindow 漏れなし。整数変換はマスク済み抽出で安全、マルチスレッド境界は単一 UI スレッド前提が一貫して守られる。挙動変更を要する対策は不要のため proposals 新規記録なし、既知 P63/P64/P65 は参照に留めた。
