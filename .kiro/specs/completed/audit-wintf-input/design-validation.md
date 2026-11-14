# 設計の検証: audit-wintf-input

- 検証した日: 2026-11-14
- 読んだもの: `spec.json`・`requirements.md`・`design.md`・`research.md`（見出しと 3 節）・steering（`product.md`・`tech.md`・`structure.md`）
- コードで確かめた所: `drag/state/mod.rs`・`drag/dispatch.rs`・`drag/accumulator.rs`・`drag/systems.rs`・`pointer/buffers.rs`・`pointer/dispatch/mod.rs`・`ecs/world/mod.rs:184-216`・`window_proc/mouse_move.rs:200-479`・`layout/hit_test/mod.rs:500-516`・`layout/systems/window_pos_systems.rs:135-169`・`window/window_handle.rs`・`window/command.rs:850`・`clickthrough/controller.rs:105`・`clickthrough/monitor.rs`、範囲の外の `areka-placement/src/balloon_lock.rs:98-114`・`balloon_lock_drag_tests.rs:61-125`・`areka/src/input_events/balloon_moved.rs`・`areka/src/runtime/balloon_hover.rs`

## まとめ

設計は 23 件の指摘をどれも直しの箇所とテストの置き場まで行の番号で示し、要件の討議の決め 1〜11（`audit-wintf-window` との分け方を含む）と食い違う所は本文には無い。指摘 1・11・18 の直し方はコードと突き合わせて正しいと確かめた。ただし指摘 3 の直し方が、範囲の外の製品コードの読み方と衝突する（下の 1）。これは小さな書き換えで直るので、判定は GO（設計の討議で 1〜3 を片付けてからタスクへ）。

## 確かめた主張

| 主張 | 結果 |
|---|---|
| 指摘 1 ⑴: 今は `cleanup_drag_state` が `dispatch_drag_events` の後に登録され（`world/mod.rs:202-206`）、`DragEndEvent` を読んで同じ entity の `DraggingState` を外す。`EcsWorld::new()`＋`world_mut().run_schedule(Input)` で組むテストは直す前に赤になる | 正しい。`EcsWorld::new` は `install_drag_accumulator` を通る（`world/mod.rs:133`）ので、終わりの種も同じ累積器に積める |
| 指摘 1 ⑵: 始まりの腕で「今の状態が同じ entity の `JustStarted`」のときだけ `update_dragging` を呼ぶ | 正しい。前の腕は `JustStarted B` を見て呼ばず、B の腕が B の控えで進める。同じ entity を掴み直したときは前の腕が先に進めるが、控えは同じ World から作るので同じ値になり、害は無い |
| 指摘 1 ⑶: `Ended` を積むときに移動量を 0 に戻す | 正しい。今は `flush` まで A の移動量が残り、B の `DragEvent` として配られる |
| 指摘 18: 守りを `CAPTURE_GUARD` へ出し、`start_preparing` は「残りを落とす → 取る」 | 正しい。落とすときの `WM_CAPTURECHANGED` の手続きは、押していない状態を見て何もしない。順を逆にすると新しい捕捉を外す、という説明も合っている。`DragStateSnapshot` が欄を全部 `*` で写していることから、`DragConstraint`・`Point`・`HWND`・`Instant` が `Copy` だと分かり、`DragState` は `Copy` にできる |
| 指摘 11: `local = client + (窓の bounds の左上 − entity の bounds の左上)` | 正しい。窓は LayoutRoot（拡大率 1）の子で `offset = WindowPos.position`（`window_pos_systems.rs:151-156`）。ドラッグ中は写さない（`Without<WindowDragging>`）ので、両方の項を同じ配置から取る形が正しい。式は窓の位置によらずクライアント座標だけで決まる |
| 指摘 11 の引く時点: tick の頭で全部の `PointerState` を書き直し、窓の手続きの作る所は触らない | 決め 11 と合っている（ただし下の 3 のとおり、タスクの表の 8 行目だけが食い違う） |
| 指摘 19 と `BalloonHover` の危うさ | 設計の書いたとおり。`on_balloon_pointer_moved` は毎回 `set_balloon_hover` を呼び（`balloon_moved.rs:114-116`）、可視性の相がそれを毎フレーム読む |
| 23 件それぞれの直しとテスト（要件 4.6） | 23 件とも「Components and Interfaces」に直しがある。バグと危ない所（1・2・3・4・11・21）と 6・8・19 にテストがあり、テストを足さない件にも 1 件ずつ理由がある |
| タスクの見込み 20 以下（要件 8.1） | 17。20 以下 |
| 指摘 9⑹ の範囲の外の書き手 13 か所 | grep で数え直して合っていた（`areka-placement` 7・`areka` 4・wintf のテスト 2。ほかの `is_primary:` はモニタの記録） |

## 大事な指摘（3 件）

### 1. 指摘 3 の「控えの `move_window` を偽にする」は、`areka-placement` の製品コードでは「止めたドラッグ」という意味になる

- **何が問題か**: `balloon_lock.rs:103-114` の `balloon_drag_held` は、控えの `hwnd` がバルーンの窓のものなら `!ctx.move_window` で「このドラッグは止めている」と決める。設計どおり位置が取れないときに控えの `move_window` を偽にすると、`DragConfig.move_window = true` のバルーンが「止めた」側へ回る。`balloon_lock_drag_tests.rs` は偽の HWND（`fake_handle`・`:61-66`）と `DragConfig::default()`（`move_window: true`）でバルーンを組んでいて、偽の HWND では `client_to_window_coords` が必ず `Err` になる（`window_handle.rs:77-84`）。そのため、走っている `audit-placement-core` のテストの期待が裏返る。
- **なぜ大事か**: 「wintf は窓を動かさない」という合図と、「areka が窓を受け持つ」という方針の値が 1 つの欄に重なっている。製品コードの判断が変わるうえ、要件 5.2（走っている spec と申し合わせてから触る）にも響く。設計は Revalidation Triggers で「実装の頭で grep する」と書くだけで、この衝突は見つけていない。
- **直し方の案**: 控えの `move_window` は今どおり `DragConfig` の値にする。位置が取れないことは、もともと `Option` の `WindowDragContext.initial_window_pos` を `None` にして伝える。`update_dragging` は `move_window = ctx.move_window && ctx.initial_window_pos.is_some()` で `DragState::Dragging` の欄を決め、`None` のときに `warn!` を出す。こうすれば `balloon_lock.rs` は変わらず、偽の HWND のテストの意味も保たれる（増えるのは `WARN` の行だけ）。`dispatch_order_tests.rs` の指摘 3 のテストは「控えの位置が `None`・`Dragging.move_window` が偽・`WARN` が 1 行」に直す。
- **関わる要件**: 2.3・5.1・5.2・4.6
- **設計の箇所**: 「指摘 3」の節・Error Handling の表 1 行目・Revalidation Triggers の「始めの位置が取れないドラッグ」・「設計の段で決めた事項」の 5

### 2. 指摘 19 がバルーンの自動で閉じる時刻を変えうるのに、決めも、それを固定するテストも無い

- **何が問題か**: 「未解決の危うさ」の 1 は、止まったカーソルの下でバルーンが出直したときに滞在の記録が付かなくなることを正しく挙げている。ただ、推しの ⑴ の根拠は「Windows は窓が出るときに移動の知らせを送るので、出直しの大半は届く」という見込みだけで、areka のバルーンが実際にどう隠れ、どう出直すか（`ShowWindow` か、描画面の不透明度か）を確かめていない。隠す仕組みが窓を隠さないもの（描画面の不透明度など）なら知らせは来ず、カーソルを乗せたままのバルーンが閉じるようになる。
- **なぜ大事か**: 外から見える振る舞い（バルーンを乗せて止める操作）が変わる。決め 9 が閉じたのは SHIORI へ送る `OnMouseMove` の数だけで、こちらは閉じていない。要件 3.1 の 3 種類目（SSP との互換）に当たるかどうかも、まだ判断していない。
- **直し方の案**: 設計の討議で ⑴ か ⑵ を決める前に、可視性の相（`balloon_visibility_phase`）がバルーンを隠して出し直すときの道を読み、移動の知らせが来るかを書き留める。来ないなら ⑵ に当たる手当てを選ぶ（例: `PointerState` を持つ entity の配置が変わった tick も `moved` を真にする）。どちらに決めても、`areka` の側（走っていないので触れる）に「止めたカーソルの下で出直したバルーンの滞在」を固定する決定論のテストを 1 本置く。
- **関わる要件**: 3.1・3.2・4.6・4.7
- **設計の箇所**: 「指摘 19」の節・Revalidation Triggers の「`OnPointerMoved` の回数」・「未解決の危うさ」の 1

### 3. タスクの表の 8 行目が決め 11 と食い違い、タスク作りを誤らせる

- **何が問題か**: 「タスクの見込み」の 8 行目は「本当のローカル座標（窓の手続きの 4 か所）」と書く。ところが決め 11 と設計の本文（File Structure Plan の範囲の外の表の下・「設計の段で決めた事項」の 10）は、窓の手続きの作る所には触らず、`transfer_buffers_to_world` の最後で書き直すと決めている。ほかにも小さな食い違いが 2 つある。Revalidation Triggers は「本番の受け手は 2 つ」と書いて 3 つ並べている（シェルの箱はキャラの窓の受け手の中から呼ばれる）。また、消す名 `CursorVelocity` を文字で挙げる `.kiro/specs/window-snap/brief.md:33` が、要件 5.3 の「文字列での参照」の確かめから漏れている。
- **なぜ大事か**: `/kiro-spec-tasks` は表をそのままタスクにするので、`audit-wintf-window` のファイルへ要らない手を入れるタスクができてしまう（決め 11・要件 5.4 に反する）。
- **直し方の案**: 8 行目を「本当のローカル座標（`buffers.rs` の `local_point_of` と `transfer_buffers_to_world` の最後の書き直し・窓の手続きは触らない）」に直す。受け手の数は「2 つ（キャラの窓・バルーンの窓。シェルの箱はキャラの窓の受け手の中）」にする。`window-snap/brief.md` を範囲の外の表に足すか、「事実の説明なので直さない」と一言書く。
- **関わる要件**: 5.3・5.4・決め 11
- **設計の箇所**: 「タスクの見込み」の表・Revalidation Triggers・File Structure Plan の範囲の外の表

## よい所

1. **範囲の外の行と持ち主が全部見える**: 範囲の外で触るファイルの 1 つずつに、持ち主の spec・走っているか・変える行・指摘の番号が付いていて、申し合わせの表と順序（先に着いた方が入れ、後の方が追う）まで決め 2・7・8・11 と揃っている。並走のぶつかりを実装の前に読める。
2. **直す順で直しを小さくしている**: 指摘 18（`DragState` を `Copy` に）を先に置くことで、指摘 1 ⑵ の比べが `matches!` 1 つで済み、テストも状態を直に組めるようになる。さらに指摘 1 のテストを、本番の登録どおりの `EcsWorld::new()`＋`run_schedule(Input)` で組むので、直す前に確かに赤になる（`cleanup_drag_state` の登録をコードで確かめた）。

## 判定

**GO**（条件つき）

- **理由**: 範囲の決め・依存の向き・C20 の約束・1,000 行の番人・タスクの数はどれも守られていて、直しの道筋ははっきりしている。大事な指摘の 1 は製品コードとの衝突だが、欄を 1 つ替えるだけで直り、形の作り直しは要らない。
- **次にすること**: 設計の討議（`/kiro-design-discussion audit-wintf-input`）で 1〜3 を議題にし、`design.md` を直す。1 は推しの案（`initial_window_pos: None` で伝える）を入れる。2 は隠す道を読んでから ⑴ か ⑵ を決め、`areka` 側のテストを足す。3 は表と説明を直す。そのあと `/kiro-spec-tasks audit-wintf-input` へ進む。
