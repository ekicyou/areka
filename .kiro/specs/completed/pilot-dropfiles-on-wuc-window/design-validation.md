# 設計レビュー: pilot-dropfiles-on-wuc-window

> 2026-09-26・`/kiro-validate-design`（非対話・サブエージェント）。対象は確定済みの `design.md`（`requirements.md` 要件 1〜7 に対する設計）と、ブランチ `claude/pilot-dropfiles-wuc-window-f3f940` の実物のコード。
> 判定の物差しは「使い捨ての先進坑（規模 S・1〜3 タスク）」。production 向けの抽象化・テスト・磨きは求めない。求めるのは、実験が brief の問いに答えること、要件 1.1〜7.6 が全部設計に載っていること、wintf・areka を触らないこと。
> 引用は「どのファイルの、何の定義か」で指す（行番号は使わない）。

## 総評

設計は実験の問い（本番と同じ様式の WUC 合成窓に `WM_DROPFILES` が届くか・透過している所では背後へ抜けるか）に最短で答える構成になっている——受け口は `SetWindowSubclass` 1 本、絵は不透明な矩形 1 つ、切替は環境変数 2 つ、変更は `crates/pilot/examples/pilot-dropfiles-on-wuc-window/` の新規 3 ファイルだけで、wintf・areka のコードにも `crates/pilot/Cargo.toml` にも触らない。要件 1.1〜7.6 は対応表で全行が埋まり、要件討議の裁定 2 件（透過の外で受け取ってしまう場合は「直す」・管理者起動は記録だけで手当てしない）も Non-Goals と見立ての表に正しく写っている。実物のコードと照らして見つかった食い違いは 3 件あるが、いずれも設計文の 1〜2 行の直しで済み、実装を止めるものではない。**判定は GO**。

## 実物との照合（設計の主張 → コードでの裏取り）

| 設計の主張 | 実物 | 結果 |
|---|---|---|
| World の取っ手は `Rc<RefCell<EcsWorld>>` | `crates/wintf/src/runtime/mod.rs` の `WinApp::world()` が `Rc<RefCell<EcsWorld>>` を返す。`EcsWorld::world(&self) -> &World`（`crates/wintf/src/ecs/world/mod.rs`） | 一致 |
| 受け口の中で `try_borrow` して判定・借りられなければ `unknown` | wintf 自身の窓手続き（`crates/wintf/src/runtime/wndproc_bridge.rs` の `make_wndproc`）が同じ作法（`try_borrow` 失敗は既定手続きへ読み飛ばす）。tick は `tick_one_frame`（`crates/wintf/src/runtime/tick_bridge.rs`）が `try_borrow_mut` で借り、実行器のタスクから回る（窓手続きの中からではない）。投函されたメッセージはライブラリのループが tick の外で配送するので、通常走行では借用は成功する | 一致（例外 1 つ＝後述の指摘 3） |
| `hit_test_in_window(world, window, client_point: PhysicalPoint)`・引数は物理 px のクライアント座標 | `crates/wintf/src/ecs/layout/hit_test/mod.rs` の `hit_test_in_window` は `client_point` に `WindowPos.position` を足してスクリーン座標にし `hit_test` へ渡す。透過機構（`crates/wintf/src/ecs/clickthrough/controller.rs` の `evaluate_targets`）も `ScreenToClient` の結果（物理 px のクライアント座標）をそのまま渡している | 単位は一致。**型名に罠**（指摘 2） |
| `DragQueryPoint` の値は物理 px（本プロセスは PMv2） | `DragQueryPoint(hdrop, *mut POINT) -> BOOL` はクライアント座標を返す。`WinApp::new` が PMv2 を設定するのでクライアント座標＝物理 px | 一致 |
| Win32 API は `windows` 0.62.2 の既定 features で供給済み | `Win32::UI::Shell`: `SetWindowSubclass(hwnd, SUBCLASSPROC, usize, usize) -> BOOL`・`DefSubclassProc`・`SUBCLASSPROC = Option<unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM, usize, usize) -> LRESULT>`・`HDROP`・`DragQueryPoint`・`DragQueryFileW(HDROP, u32, Option<&mut [u16]>) -> u32`・`DragFinish(HDROP)`・`DragAcceptFiles(HWND, bool)`・`IsUserAnAdmin() -> BOOL`。`Win32::UI::WindowsAndMessaging`: `WM_DROPFILES = 563`・`WS_EX_ACCEPTFILES = WINDOW_EX_STYLE(16)`・`GetWindowLongPtrW`／`SetWindowLongPtrW`／`SetWindowPos`。ルート `Cargo.toml` の `[workspace.dependencies.windows]` の features に `Win32_UI_Shell`・`Win32_UI_WindowsAndMessaging` あり | 一致（`crates/pilot/Cargo.toml` の変更は不要） |
| `compute_ex_style` は `WS_EX_LAYERED` を落とし `WS_EX_NOREDIRECTIONBITMAP` を足すだけ | `crates/wintf/src/runtime/window_factory.rs` の `compute_ex_style`＝`(style.ex_style & !WS_EX_LAYERED) \| WS_EX_NOREDIRECTIONBITMAP` | 一致（`WS_EX_ACCEPTFILES` は通る） |
| 生成後の `SetWindowSubclass` はライブラリの手続きと衝突しない | `wintf-winmsg-executor` 0.0.5 `src/util/window.rs`: `WM_NCCREATE` で `GWLP_WNDPROC` を差し替え `GWLP_USERDATA` に状態を置く。`WM_NCDESTROY` で `Box::from_raw` で解放するだけで `GWLP_WNDPROC` は戻さない | 一致 |
| 裁定 (1) 透過の外で受け取る＝「直す」 | design.md「見立て」の表 3 行目「ⓐ・ⓒで届くがⓑで窓が受け取る → **直す**」・Out of Boundary に「捨てる処理は本坑」 | 要件 5.7 に一致 |
| 裁定 (2) 管理者起動の手当ては扱わない・起動時に記録だけ | Non-Goals に `ChangeWindowMessageFilterEx` を対象外と明記・Key Decision 8 で `IsUserAnAdmin` を起動の行に 1 回 | 要件 5.2 に一致 |
| wintf・areka を触らない | Modified Files「なし」・Allowed Dependencies は pilot → 既存 crate の一方向のみ | `two-tunnel.md` の隔離に一致 |

## 指摘（最大 3 件・設計討議へ）

### 指摘 1: World の取っ手は新しく作らず、wintf が既に入れている `EcsWorldSelfRef` を使う

- **問題**: 設計は `WorldHandle(Rc<RefCell<EcsWorld>>)` を新しい NonSend リソースとして `run()` の前に World へ入れ、`on_window_created` から読んで受け口の文脈に渡す。ところが wintf は `run()` の冒頭（`crates/wintf/src/runtime/mod.rs` の `wire_new_path`）で同じ目的の `EcsWorldSelfRef(Weak<RefCell<EcsWorld>>)`（`crates/wintf/src/ecs/world/mod.rs`・公開）を World へ入れており、areka 本体も同じ作法で使っている（`crates/areka/src/menu/trigger.rs` の `poll_menu_query`＝`get_non_send::<EcsWorldSelfRef>()` → `upgrade()` → `try_borrow_mut()`）。
- **影響**: 部品が 1 つ増えるだけでなく、`Rc` を World 自身の中に入れるので World が自分を強く指す輪ができ、`EcsWorld` がプロセスの終わりまで解放されない（使い捨てなので致命ではないが、終了時の後片付けの観測＝research の R5 を濁らせる）。
- **提案**: `Context.world` を `Weak<RefCell<EcsWorld>>` にし、`on_window_created` が `NonSend<EcsWorldSelfRef>` から `.0.clone()` で複製して渡す。`handle_drop` は `upgrade()` → `try_borrow()` の順で試し、どちらかが失敗したら `opaque=unknown`。`WorldHandle` と「`run()` の前に入れる」手順は削る。
- **要件**: 3.2（絵の上か外か）・2.8（物理 px のまま判定）
- **設計の場所**: Key Decisions 3・Runner「Responsibilities & Constraints」の `WorldHandle`・DropReceiver「Service Interface」の `Context`

### 指摘 2: `PhysicalPoint` は wintf に 2 つあり、`wintf::ecs::PhysicalPoint` は整数の `Point`——設計は `PointF` と書く

- **問題**: 設計は「`PhysicalPoint` は `PointF` の別名」とし `PhysicalPoint::new(x as f32, y as f32)` を `hit_test_in_window` に渡すと書く。実物では別名が 2 つある: `crates/wintf/src/ecs/layout/hit_test/mod.rs` の `pub type PhysicalPoint = PointF`（`hit_test_in_window` の引数）と、`crates/wintf/src/ecs/pointer/types/mod.rs` の `pub type PhysicalPoint = Point`（整数）。`crates/wintf/src/ecs/mod.rs` は `pub use layout::*`（glob）と `pub use pointer::{…, PhysicalPoint, …}`（明示）の両方を持ち、明示の方が勝つので **`wintf::ecs::PhysicalPoint` は整数の `Point`** になる（透過機構の `controller.rs` が `crate::ecs::PhysicalPoint` を `POINT { x: screen.x, .. }` に直接入れているのがその証拠）。
- **影響**: `use wintf::ecs::PhysicalPoint` と書くと `new(f32, f32)` が無くて組めない。コンパイラが即座に止めるので実害は小さいが、設計文の記述が実物と食い違っている。
- **提案**: 設計の該当箇所を `PointF::new(x as f32, y as f32)`（`wintf::ecs::PointF`）に書き換え、「`hit_test_in_window` の引数の型は `PointF`」と明記する。単位（物理 px のクライアント座標）の主張はそのままで正しい。
- **要件**: 2.8・3.2
- **設計の場所**: Existing Architecture Analysis 4 点目・DropReceiver「Responsibilities & Constraints」の `handle_drop` ③・Requirements Traceability 2.8 の Interfaces 列

### 指摘 3: `opaque=unknown` を「同期配送の証拠（R1）」と読むには条件が要る

- **問題**: 設計は「`try_borrow` が失敗する経路は tick の途中で `WM_DROPFILES` が同期配送されるときだけ」「起きたら `opaque=unknown` の行が R1 の答えになる」と書く。World が借りられているときに投函済みメッセージが配送される経路が wintf にもう 1 つある: 終了処理の `drain_dispatcher_queue`（`crates/wintf/src/com/wuc.rs`）の中の `pump_current_thread_messages` が `PeekMessageW(PM_REMOVE)`＋`DispatchMessageW` で待ち行列を空にする。終了の瞬間に落とし物が届けば、同期配送でなくても `unknown` が出る。
- **影響**: 成果物は知見なので、`unknown` を無条件に「同期配送あり」と README に書くと学びが誤る（可能性は低いが、上限時間で終わる example では終了の瞬間に手が動いていることはあり得る）。
- **提案**: 設計の Risks と README の学びに「`unknown` が R1 の証拠になるのは `[dropfiles] 終了` の行より前に出たときだけ」と 1 行足す。コードの変更は不要。
- **要件**: 7.5（学び）・3.5
- **設計の場所**: Key Decisions 3 の末尾・DropReceiver「Implementation Notes」の Risks・research.md §10.4

## 設計の強み

1. **問いに対して最短で、しかも本番と同じ判定器で測る**: 受け口を `SetWindowSubclass` 1 本に絞り、絵の上か外かを透過機構と同じ `hit_test_in_window`・同じ座標系で判定するので、要件 4 の「クリックの当たり判定と一致するか」を別の物差しを持ち込まずに測れる。案 C（メッセージ取得フック）と待ち行列を先に作らず「ⓐで届かないと分かったときだけ足す」と決めた点も、使い捨ての規律に合っている。
2. **要件と裁定の写しが正確**: 対応表が 1.1〜7.6 の全行を埋め、見立ての表が要件 5.3〜5.7 をそのまま写している（ⓑで抜けない＝「直す」・手当てを尽くしてⓐかⓒで届かない＝「違う」）。管理者起動は Non-Goals で明示的に外し、起動の行 1 つで判定から除外できる形にしてある。

## 細かな観察（直すなら実装時に・討議は不要）

1. `ExitReason::InitFailure` を「窓が外から閉じられた」場合（Alt＋F4 → wintf 既定の閉じ要求 → 窓の despawn → `run()` が戻る・`Run.exit == None`）にも使っている。要件 6.4 は終了の理由を求めるので、`ExternalClose` のような別の名で `warn!` に出す方が読み違えない。
2. `catch_unwind` に `&Context` を渡すには `AssertUnwindSafe` で包む必要がある（1 行）。
3. `Fix::Reapply` は生成直後の `ex-style`（`when=created`）の行で `accept_files=true` なら何も変えない手当てになる（`compute_ex_style` はビットを通す）。README に「`created` の行でビットが無いときだけ意味を持つ」と書いておくと、試して無意味だったと後で分かる手戻りが減る。
4. README の既定ログの説明に「`RUST_LOG` を自分で設定すると既定の `wintf::ecs::clickthrough=debug` は消えるので、設定するなら `info,wintf::ecs::clickthrough=debug` を含める」と書く（要件 2.7）。
5. README の手順ⓑ: ドラッグの起点のエクスプローラをクリックした瞬間にその窓が前面に来る。ドラッグ中は先進坑の窓を前に出せないので、「起点の窓は先進坑の窓と重ならない所に置き、受け手の窓だけを余白の下に置く」と手順に書く。
6. 拡張スタイルの読み戻しは `WindowHandle::get_style`（公開）でもできるが、到着時は HWND しか手元に無いので `GetWindowLongPtrW` 直呼びで統一する設計の判断は妥当。記録のみ。

## 判定

**GO**。

- **理由**: 実験が brief の問いに直結し、要件 1.1〜7.6 が全部設計に載り、wintf・areka・他 crate の `Cargo.toml` に触らない。指摘 3 件は設計文の 1〜2 行の直し（取っ手の型を `Weak` にして `EcsWorldSelfRef` を使う・`PointF` と書く・`unknown` の読み方に条件を付ける）で済み、構造には影響しない。
- **次の一手**: 設計討議で指摘 1〜3 を design.md に反映してから `/kiro-spec-tasks pilot-dropfiles-on-wuc-window` へ。タスクは 3 本以内（窓と器／受け口と手当て／README）で足りる。
