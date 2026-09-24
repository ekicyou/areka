# ギャップ分析: pilot-balloon-asset-swap

- 分析日: 2026-09-24
- 対象: `requirements.md`（生成済み・未承認）と、現在のコード（ブランチ `claude/pilot-balloon-asset-swap-04467f`・`c90527dd`）
- 前提: 本 spec は先進坑（使い捨て・`.kiro/steering/two-tunnel.md`）。成果物は知見であり、ここに書くのは「実験を組むときに何が既に在り、何が無く、どこで要件がコードの形とぶつかるか」である。最終の決定はしない。

---

## 1. 要約

- **素の `attach_target` の再登録では、古い絵が消えない構造である（コードから読める）。** 再登録は presenter が持つ表示コンテキストを丸ごと差し替えるが、古い装着（窓の子の 2 つの entity）を消す処理がどこにも無い。古い子は見えるまま・当たり判定も効くまま窓の下に残り、新しい子はその後ろに足される。基準の版（要件 5.1）は実行する前から「古い絵の残り」と「混在」が出ると読める。
- **さらに wintf では、兄弟の重なり順が「描画」と「当たり判定」で逆になっている。** 描画は先頭の子が一番上、当たり判定は最後の子から調べる。古い子と新しい子が並ぶと「絵は古いバルーンが上・当たり判定は新しいバルーンが優先」になり、混在がずっと続く形になる。
- **「実際に描画された結果の読み戻し」（要件 3.2）に使える公開の手段は、プロセスの中には無い。** `EmoPresenter::read_back` は合成メモ（表示へ渡した原寸の値）を返すだけで、画面の絵ではない。画面の絵を取るには OS の画面取り込み（Desktop Duplication など）を pilot 側で組む必要がある。当たり判定は wintf の公開関数 `hit_test_in_window`（クリック透過の制御が使うのと同じ関数）に標本点を問い合わせれば「実際に効いている判定」を取れる。
- **検体 2 つは寸法・色・α の形のどれでも見分けられる。** `StayseeBalloon` の `balloons0.png` は 335×205・淡い青白、`emo2-kakukaku` の `balloons0.png` は 400×224・純白に黒縁。共通範囲 335×205 の中でも「片方だけ不透明」な画素が 5,017 と 2,554 ある。
- **要件 5.2 の「当たり判定を先に替える版／絵を先に替える版」は、presenter の公開の口では分けられない。** `apply(ShowSurface)` が表示記録とマスクを同じ呼び出しの中で差し込むためである。意味のある版の切り方は別にある（§6 議題 2）。規模は S の上限、危険度は中（観測の組み方が未知）。

---

## 2. 現状調査

### 2.1 `EmoPresenter::attach_target` の再登録で実際に起きること

根拠: `crates/areka-emo-present/src/presenter/hub.rs` の `attach_target` の定義、`crates/areka-emo-present/src/presenter/target.rs` の `PresentTarget` の定義、`crates/areka-emo-present/src/mount.rs` の `VisualMount` の定義、`crates/areka-emo-present/src/presenter/show.rs` の `apply_show` の定義。

| 問い | コードから分かること |
|---|---|
| 同じ id を再登録すると何が起きるか | `self.targets.insert(target, PresentTarget { .. })` で表示コンテキストを丸ごと差し替える。新しいコンテキストは `mount: None`・`visible: false`・`current_surface_id: None`・`applied: None`・`native_size: None`・`last_show: None`・`pending_resize: None`・キャッシュ空・`ownership` は既定（`CommandDriven`）。World には一切触れない（引数の `_world` は使われていない）。 |
| 古い visual／entity は消えるか | **消えない。** 古い `PresentTarget` は `HashMap::insert` の戻り値として捨てられるが、`VisualMount` にも `PresentTarget` にも `Drop` の実装が無い（`crates/areka-emo-present/src` 全体で本番コードの `impl Drop` は 0 件・`despawn` も 0 件）。窓の子として spawn された `emo-surface` と `emo-text-layer-slot` の 2 entity は World に残る。 |
| 残った古い子の状態 | 最後の `ShowSurface` で `set_visible(true)` された状態のまま。`Visual.is_visible = true`、`HitTest::alpha_mask()`、`AlphaMaskResource` には古いバルーンのマスク、`GraphicsCommandList` には古い絵の表示記録。以後この子を触る口は presenter から失われる（`surface_entity()`・`text_slot()` は `pub(crate)`、新しいコンテキストは古い mount を知らない）。 |
| 次の `ShowSurface` まで窓はどう見えるか | 新しいコンテキストの `mount` は `None` なので新しい絵はまだ無い。しかし古い子が残っているので窓は古い絵を出し続ける（「空」にはならない）。当たり判定も古いマスクで効き続ける。 |
| 次の `ShowSurface` で何が起きるか | `mount.is_none()` の分岐で `VisualMount::attach` が**新しい** 2 entity を窓の子として spawn する。表示記録・配置・`AlphaMaskResource::set_shared`・可視化は同じ呼び出しの中で続けて行われる（`apply_show` の手順 (3)＝「表示記録 → 配置 → マスク同期 → 可視化」）。窓の子は `[古い文字層, 古い面, 新しい文字層, 新しい面]` の 4 つになる。 |
| 重なり順（描画） | `crates/wintf/src/ecs/graphics/systems/visual_sync.rs` の `visual_hierarchy_sync_system` は子を前から順に `InsertAtBottom` する＝**先頭の子ほど上**。古い面が新しい面の上に描かれる。新しいバルーン（400×224）の方が大きい向きでは、古い絵（335×205）が上に乗り、周りに新しい絵がはみ出して見える。 |
| 重なり順（当たり判定） | `crates/wintf/src/ecs/common/tree_iter.rs` の `DepthFirstReversePostOrder` は子を前から積んで後ろから取り出す＝**最後の子から先に調べる**（コメントも「最後の要素（最前面）」と書く）。`crates/wintf/src/ecs/layout/hit_test/mod.rs` の `hit_test` は最初に当たった entity を返すので、新しい面のマスクが優先され、そこが透明なら古い面のマスクで当たる。 |
| 描画と当たり判定の食い違い | 上の 2 行により、古い子が残る限り「絵は古いバルーンが上」「当たり判定は新しいバルーン優先（外れた所は古いバルーン）」となる。**素の再登録は、混在と古い絵の残りを構造として持ち続ける。** これは 1 フレームの話ではなく、古い子を誰かが消すまで続く。 |
| 文字層の古い子 | `emo-text-layer-slot` は `HitTest::bounds()` で可視のまま残る。`Visual` の `on_add` が既定の `Arrangement`（`crates/wintf/src/ecs/graphics/visual.rs` の `on_visual_add`）を入れるだけなので寸法は 0 と読め、当たり判定への影響は無いと見込むが、実測はしていない（設計で確かめる事項）。 |
| 可視性の所有者 | 再登録は `ownership` を既定（`CommandDriven`）へ戻す。本番のバルーン窓は `External`（`crates/areka/src/emo2_boot/frame/attach.rs` の `set_visibility_ownership` 呼び出し）なので、本坑で再登録を使うなら所有者を付け直す手順が要る。本先進坑は文字もバルーンの可視性制御も持たないので、`CommandDriven` のままで実験は組める。 |
| 窓寸の合わせ直し | 再登録は `applied`／`native_size` を `None` へ戻すので、次の `ShowSurface` は「初回」扱いで必ず `pending_resize` を積む（`apply_show` の手順 (3a)／(3.5)）。335×205 と 400×224 の往復では窓の client 寸も毎回変わる。 |

**同じフレームの中で絵と当たり判定がどう切り替わるか**（根拠: `crates/wintf/src/ecs/world/mod.rs` の `try_tick_world` と、同ファイルの既定スケジュール登録）:

1 回の tick は `Input → Update → PreLayout → Layout → PostLayout → UISetup → GraphicsSetup → Draw → PreRenderSurface → RenderSurface → Composition → CommitComposition → FrameFinalize` の 13 段を順に回す。

- 当たり判定に要る `GlobalArrangement` は `PostLayout` の `propagate_global_arrangements` で決まる。`hit_test_entity` は `GlobalArrangement` の無い entity を「当たらない」とする。
- 新しい面の WUC visual と描画面は `PreRenderSurface`（`visual_resource_management_system`・`deferred_surface_creation_system`）で作られ、`RenderSurface`（`render_surface`）で描かれ、`Composition`（`visual_hierarchy_sync_system`・`visual_property_sync_system`）で木へ繋がり可視性が反映される。WUC への反映は明示の commit が無く、DispatcherQueue 経由の暗黙の反映である（`crates/wintf/src/ecs/graphics/systems/render.rs` のコメント・`crates/wintf/src/ecs/world/mod.rs` の `CommitComposition` の注記）。
- **差し替えをどの段で呼ぶかで、1 tick のずれが構造として生まれるかが決まる。**
  - 手本 `crates/areka/examples/emo-present.rs` は `boot_present_system`・`cycle_present_system` を `FrameFinalize`（コミットの後）へ載せている。ここで差し替えると、World（当たり判定の元）はその場で変わるが、絵は次の tick の `PostLayout`〜`Composition` まで変わらない。tick と tick の間に当たり判定を問い合わせると、「古い絵のまま・当たり判定は新しい状態（または新しい面は `GlobalArrangement` がまだ無いので当たらない）」が 1 tick 分できる。
  - 本番の `emo2_frame_system` は `Update` に載っている（`crates/areka/src/emo2_boot/mod.rs` の `add_systems(Update, emo2_frame_system.after(update_typewriters))`）。`Update` で差し替えれば、同じ tick の `PostLayout` で `GlobalArrangement` が決まり、同じ tick で描画・木への接続まで進む。tick の境目から見れば、絵と当たり判定が同じ tick で切り替わる形になる（ただし WUC の暗黙の反映が 1 tick の変更をまとめて 1 回の合成に載せるかは未確認＝§7 調べ物 1）。
- 以上から、**0 フレームでの差し替えが構造として成り立ち得るのは「`Update` など `PostLayout` より前の段で、古い子を消す（または隠す）ことと新しい面の装着を同じ呼び出しの中で行う」形に限られる**と読める。1 フレーム遅らせて辻褄を合わせる解は、プロジェクトの方針（記憶 no-frame-delay-fixes-change-the-state-shape）と要件 5.4 により採れない。

### 2.2 当たり判定の実体

当たり判定と呼べるものが 2 つある。要件 3.3 の「実際に効いている判定」がどちらを指すかで観測が変わる。

1. **クリック透過の判定（α マスク）**: 窓の子の `HitTest::alpha_mask()` と `AlphaMaskResource`。判定関数は `crates/wintf/src/ecs/layout/hit_test/mod.rs` の公開関数 `hit_test_in_window`（窓の client 物理座標 → 当たった entity）。クリック透過の制御（`crates/wintf/src/ecs/clickthrough/controller.rs` の `evaluate_targets`）がまさにこの関数を呼び、結果で `WS_EX_TRANSPARENT` を付け外しする。`wintf::ecs::hit_test_in_window` として外から呼べる（`crates/wintf/src/ecs/layout/mod.rs` の `pub use hit_test::*`）。`&World` だけを取るので、pilot が各 tick の後に標本点を問い合わせられる。
2. **当たり判定領域（collision の名前）**: `EmoPresenter::hit_region_client`（`crates/areka-emo-present/src/presenter/hit.rs`）。バルーンは `build_balloon_target_from_faces` が合成した surfaces.txt から組まれ、collision を持たないので、バルーンでは常に `None` を返す。**バルーンの見分けには使えない。**

OS 側の実際のクリック透過（`WS_EX_TRANSPARENT` の付け外し）は、カーソルの動きと VSync の起床で動く非同期ループなので、tick と揃わない。要件の「実際に効いている判定」は 1 の関数を指すと読むのが自然だが、要件に明記が無い（§6 議題 5）。

標本点の取り方: 2 つのマスクが食い違う画素がある（§2.4）ので、「Staysee だけ不透明」「kakukaku だけ不透明」「両方不透明」「両方透明」の代表点を数点ずつ決めておけば、`hit_test_in_window` の当たり外れの組み合わせで「どちらのマスクが効いているか」を判別できる。

### 2.3 描画結果の読み戻し手段

| 手段 | 在処 | 要件 3.2（入力ではなく描画結果）を満たすか |
|---|---|---|
| `EmoPresenter::read_back` | `crates/areka-emo-present/src/presenter/read.rs` の `read_back` の定義（公開） | **満たさない。** 合成メモが持つ原寸のバイト列（表示へ渡した値）を返す。doc にも「表示面は書き込み専用であり読み戻さない」とある。しかも再登録で新しいコンテキストの `last_show` は `None` なので、再登録の直後は失敗を返す。古い子の絵は見えない。 |
| `presenter_test_support.rs` の `make_world_with_gpu` | `crates/areka-emo-present/src/presenter_test_support.rs`（`#[cfg(test)]`） | 使えない（テスト専用・窓も画面も無い）。ただし中身は公開 API（`GraphicsCore::new`・`WucGraphicsResource::new`）の組み合わせなので、真似はできる。画面に出ない点で要件 3.7 と合わない。 |
| WUC の描画面を読む | wintf 内部 | 使えない。`CompositionDrawingSurface` は書き込み専用（記憶 areka-gpu-window-screenshot-readback）。 |
| 窓の DC から `PrintWindow`／`BitBlt` | OS | 使えない。窓は `WS_EX_NOREDIRECTIONBITMAP` で、窓単位の取り込みは黒か空になる。 |
| 画面全体の取り込み（画面 DC の `BitBlt`） | OS（`Win32_Graphics_Gdi` は workspace 既定で有効） | 合成後の画面は撮れる（完了 spec `default-balloon-bundle` で開発者が確認済み）。ただし DWM の合成フレームと揃わず、1 フレームだけの崩れを取りこぼす恐れがある。 |
| Desktop Duplication（`IDXGIOutputDuplication`） | OS（`Win32_Graphics_Dxgi` が workspace 既定に無い＝pilot の `Cargo.toml` で足す） | 満たす見込み。DWM が画面を更新するたびに 1 枚ずつ受け取れ、`AccumulatedFrames` が 2 以上なら「取りこぼした」と分かる＝要件 3.6 の「測れない」に繋げられる。 |
| Windows.Graphics.Capture（窓単位） | OS（WinRT） | 満たす見込み。窓単位で合成後の絵を取れるが、組み立てが重い。 |

**結論: 要件 3.2 を満たすには、pilot 側で OS の画面取り込みを組むことが要る（既存の crate には無い）。** 取り込みは実際の画面を見るので、窓が他の窓に覆われていないこと（手本と同じく `WS_EX_TOPMOST`）と、取り込んだ画面上の窓の位置の割り出しが要る。

### 2.4 検体 2 つの取得と見分け

- 取得: `crates/sample-ghost-kit/src/lib.rs` の窓口。
  - `StayseeBalloon`: 登記表 `SAMPLES` に `kind: SampleKind::Balloon` で在る。`SampleRoot::acquire("StayseeBalloon")?.folder()` が `<根>/balloon/StayseeBalloon/` を返す。
  - `emo2-kakukaku`: `SampleRoot::acquire("emo2")?.balloon("emo2-kakukaku")?`（手本 `crates/areka/examples/emo-present/fixture.rs` の `emo2_balloon` と同じ形）。
  - どちらも `SampleRoot` を束縛している間だけパスが生きる（`Drop` で複製の木が消える）ので、実験の間は 2 つとも値を持ち続ける必要がある。`sample-ghost-kit` は既に `crates/pilot/Cargo.toml` の `[dev-dependencies]` に在る。
- 組み立て: `areka_emo_present::build_balloon_target(balloon_dir, &decoder, 0)` が `(EmoWorld, AtlasTable)` を返し、そのまま `attach_target` へ渡せる（手本 `emo-present/setup.rs` と同じ経路）。
- 見分け（`vendors/sample_ghost/` の `.nar` の中の `balloons0.png` を直接読んで測った値）:

| | StayseeBalloon `balloons0.png` | emo2-kakukaku `balloons0.png` |
|---|---|---|
| 寸法 | 335×205 | 400×224 |
| 不透明（α≧128）の画素 | 57,629 | 62,242 |
| 多い色 | (241,248,250) など淡い青白のグラデーション | (255,255,255) 純白 53,752・(0,0,0) 黒 5,947 |
| 中央の画素 | (246,250,251,255) | (255,255,255,255) |

  共通範囲 335×205 の中で「Staysee だけ不透明」が 5,017 画素、「kakukaku だけ不透明」が 2,554 画素。加えて kakukaku は 335 列目より右・205 行目より下にも絵がある。**絵（色と外形）も当たり判定（α マスク）も見分けられる。** ただし中央付近の色差は 4〜14 程度と小さいので、絵の判別は「色の一致を画素ごとに見る」より「外形（どこに不透明な画素があるか）と縁の黒線の有無」で見る方が取り込みの揺れ（色の変換・拡大）に強い。拡大率 k≠1 の画面では取り込み側の寸法も k 倍になる。

### 2.5 器と前例

- `crates/pilot/examples/_template/`: `README.md`（3 幕の雛形）と `main.rs`（依存 0 の `println!` 1 行）だけ。写して中身を差し替える。
- 前例 `crates/pilot/examples/pilot-clickthrough-alpha-toggle/`: `main.rs`（1,006 行・DComp を自前で組んだ単一ファイル）・`README.md`（3 幕・判定は開発者が記入）・`REPORT.md`（試験項目ごとの合否台帳・「README が結論・REPORT が根拠」）。wintf にも areka の crate にも依存せず、Win32 と DComp を直に叩いている。本先進坑は presenter の `attach_target` そのものを試すので、この前例と違い既存のライブラリ crate に依存する必要がある。
- `crates/pilot/Cargo.toml`: `[dependencies]` に探索用の依存（`wintf-winmsg-executor`・`event-listener`・`windows` の追加 feature）、`[dev-dependencies]` に `sample-ghost-kit`。本先進坑が要る `wintf`・`areka-emo-present`・`areka-emo-atlas`（`WicDecoderArm`）・`areka-emo-compose`（`BindSet`・`PatternState`）・`bevy_ecs`・`tracing`・`tracing-subscriber` は未登録。example だけが使うので `[dev-dependencies]` へ置けば lib は空のまま保てる。`windows` の `Win32_Graphics_Dxgi` feature（Desktop Duplication を使う場合）も足す必要がある。いずれも pilot 側が依存するだけで、葉ノードの隔離は崩れない。
- 有界な自動終了の前例: `crates/areka/examples/collision-probe/smoke.rs` の `install_smoke_exit`（`AREKA_APP_SMOKE_EXIT_MS` を読み、`spawn_local` のタイマーで窓を消して終わる）。本先進坑は「観測が終わったら上限を待たずに終わる」（要件 6.4）と「0 以外の終了コード」（要件 2.5）も要るので、`WinApp::run` の後で終了コードを決める形を足すことになる。

### 2.6 手本 example（`crates/areka/examples/emo-present.rs` とそのフォルダ・計 1,271 行）

- 使える部分: 窓の作り方（`emo-present/window.rs`・`WS_POPUP | WS_VISIBLE`・`WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST`・窓自身は `HitTest::none()`）、GPU 資源が揃うのを待ってから `attach_target → apply(ShowSurface)` する起動の段取り（`emo-present/systems.rs` の `boot_present_system`）、窓寸の合わせ直し（`emo-present/reconcile.rs` の `reconcile_present_sizes`＝`take_pending_resize` を読んで `WindowPos` を書く）、クリック透過への窓の登録（`register_click_through_windows`）、検体の取り方（`emo-present/fixture.rs`）。
- 要らない部分: シェル窓・まばたきの巡回・起動時の golden 照合・offset の読み取り。バルーン窓 1 つに絞れば、写す量は数百行に収まる見込み。
- 注意: 手本は差し替えを `FrameFinalize` に載せている。そのまま写すと §2.1 の「1 tick のずれ」を実験に持ち込む。

---

## 3. 要件と資産の対応

凡例: **既存**＝そのまま使える／**不足**＝pilot で書く必要がある／**未知**＝設計か実験で確かめる／**制約**＝既存の形から来る縛り

| 要件 | 対応する資産 | 区分 | 補足 |
|---|---|---|---|
| 1.1〜1.6 隔離 | `crates/pilot/` の構造（空 lib・examples だけ） | 既存 | 依存の追加は `crates/pilot/Cargo.toml` の `[dev-dependencies]` へ。 |
| 2.1 検体の取得と表示 | `sample-ghost-kit`・`build_balloon_target`・手本の窓と起動 | 既存 | `SampleRoot` を実験の間ずっと持つ。 |
| 2.2 往復の差し替え | `EmoPresenter::attach_target`＋`apply(ShowSurface)` | 既存（ただし制約あり） | 素の再登録は古い子を残す（§2.1）。 |
| 2.3 窓もプロセスも作り直さない | 同上 | 既存 | 窓 entity はそのまま渡し直せる。 |
| 2.4 差し替えのログ | `tracing` | 不足（数行） | |
| 2.5 失敗時に 0 以外で終わる | — | 不足 | `WinApp::run` の後で終了コードを決める形が要る。 |
| 3.1 要求の直前から揃った後の一定数まで途切れなく観測 | — | 不足・未知 | 「フレーム」を tick で数えるか DWM の合成で数えるかが決まっていない（§6 議題 3）。 |
| 3.2 描画結果の読み戻し | OS の画面取り込み | 不足・未知 | presenter の `read_back` は入力相当で使えない（§2.3）。 |
| 3.3 絵と当たり判定がどちらのものか | 当たり判定＝`wintf::ecs::hit_test_in_window`（既存）／絵＝取り込み画像の判別（不足） | 一部既存 | 標本点の選定は §2.2。 |
| 3.4 3 種の崩れを数える | — | 不足 | 分類の規則を書く。「どちらでもない」当たり判定の扱いが未定（§6 議題 6）。 |
| 3.5 0 も明示したログ | — | 不足（数行） | |
| 3.6 見分けられないときは「測れない」 | Desktop Duplication の `AccumulatedFrames` など | 未知 | 取りこぼし・窓の覆い隠し・拡大率での崩れを「測れない」へ回す規則が要る。 |
| 3.7 実際の画面に出し続ける | 手本の窓 | 既存 | |
| 4.1〜4.7 較正 | — | 不足・未知 | 崩れたフレームをどう作るか（§6 議題 7）。「古い絵の残り」は素の再登録そのもので、「空」は `Hide` で、実際の画面に作れる見込み。「混在」は pilot が World の `AlphaMaskResource` を直接差し替えれば作れる。 |
| 5.1 基準の版 | `attach_target` の再登録 | 既存 | 実行前から崩れが出ると読める（§2.1）。 |
| 5.2 順序を変えた版 | — | 制約 | presenter の公開の口では「当たり判定だけ先」「絵だけ先」に分けられない（`apply_show` が表示記録とマスクを同じ呼び出しで入れる）。意味のある版は別に切る必要がある（§6 議題 2）。 |
| 5.4 1 フレーム遅らせる版を扱わない | — | 制約 | 方針どおり。 |
| 5.5 既存 crate の変更が要るなら学びとして記録 | — | 該当の見込みあり | 古い子を消す口が presenter に無い（§2.1）。pilot が World から名前で探して消すことは crate を変えずにできるが、それを「既存の crate を変えずに組めた」と数えるかは判断が要る（§6 議題 1）。 |
| 6.1〜6.6 有界な自動終了 | `collision-probe/smoke.rs` の前例 | 既存（形のみ） | 「観測完了で即終了」と「打ち切りの報告」は足す。 |
| 7.1〜7.7 README 3 幕 | `_template/README.md`・前例の README と REPORT | 既存 | 数の台帳を REPORT に分けるかは任意。 |

---

## 4. 実装の選択肢

差し替えの部品は `attach_target` に決まっているので、選択肢の差は主に **「観測をどこまで実物に寄せるか」** と **「どの版を比べるか」** にある。

### 案 A: 手本を縮めて写し、観測はプロセスの中で完結させる

- 手本からバルーン窓 1 つと起動の段取りを写し、差し替えの system を足す。
- 絵の判別は World の状態（窓の子のうち可視な `emo-surface` がいくつあるか・それぞれの `GraphicsCommandList` がどちらの表示記録か・`GlobalArrangement` の寸法）で行い、当たり判定は `hit_test_in_window` で行う。
- 利点: 依存の追加が最小。tick ごとに決定論的に数えられる。S に収まる。
- 欠点: **要件 3.2（入力ではなく描画結果）を満たさない。** World の状態は「描くように頼んだもの」であり、WUC の暗黙の反映が 1 回の合成にまとまるか（空の面が 1 回だけ見えないか）は分からない。

### 案 B: 手本を縮めて写し、観測は OS の画面取り込み（Desktop Duplication）で行う

- 案 A の器に、画面の更新ごとに 1 枚ずつ受け取る取り込みを足す。取り込んだ画像から窓の位置の矩形を切り出し、外形と縁の線で「Staysee／kakukaku／両方／どちらでもない」を判別する。
- 当たり判定は各 tick の後に `hit_test_in_window` を標本点で問い合わせ、取り込み画像と時刻で突き合わせる。
- 利点: 要件 3.2 を文字どおり満たす。1 回の合成に空の面が挟まる崩れも見える。
- 欠点: tick と DWM の合成フレームの対応付けが要る（どちらの時刻で「同じフレーム」と言うか）。窓が覆われる・拡大率・色の揺れで判別が崩れる（「測れない」へ回す規則が要る）。`windows` の feature 追加と取り込みの組み立てで、規模は S の上限。

### 案 C: 両方を持ち、数えるのは tick、確かめるのは画面（ハイブリッド）

- 主な数は案 A の tick ごとの判別で出し、同じ走行で案 B の取り込みを並べて「tick で 0 と数えた区間に、画面で崩れが見えたか」を照らす。
- 利点: tick の数え方の見落としを画面側で拾える（要件 4 の較正を 2 段で見られる）。
- 欠点: 作る物が一番多い。先進坑の規模 S を超える恐れがある。

### 比べる版の候補（どの案でも共通）

| 版 | 中身 | 既存 crate を変えるか | 予想 |
|---|---|---|---|
| (i) 素の再登録（基準・要件 5.1） | 同じ id で `attach_target` → `apply(ShowSurface)` | 変えない | 古い絵が上に残り、当たり判定は新しいマスク優先＝混在と残りが続く（§2.1）。 |
| (ii) 再登録＋古い子の除去 | 再登録の前に、pilot が窓の子の `emo-surface`／`emo-text-layer-slot` を World から探して despawn し、同じ呼び出しの中で再登録と `ShowSurface` を行う | 変えない（ただし presenter の内部の entity に外から手を入れる） | 古い子の除去と新しい子の装着が同じ tick に入れば、崩れ 0 の見込み。消えた visual が WUC の木から外れる時期は未確認。 |
| (iii) 別 id で装着し、古い id を隠す | 同じ窓に `TargetId` を新しく割り当てて `attach_target`＋`ShowSurface`、同じ呼び出しの中で古い id へ `Hide` | 変えない | 古い子は `HitTest::none()`・不可視になるので混在は消える見込み。古い子は残り続ける（往復のたびに増える）。 |
| (iv) 差し替えを置く段 | 上の各版を `Update` に置くか `FrameFinalize` に置くか | 変えない | `FrameFinalize` では絵が 1 tick 遅れる（§2.1）。`Update` なら同じ tick。 |

要件 5.2 が挙げる「当たり判定を先に替える版」「絵を先に替える版」は、presenter の公開の口では組めない（`apply_show` の手順 (3) が表示記録・配置・マスク・可視化を 1 回の呼び出しで行う）。組むには pilot が World の `AlphaMaskResource`／`GraphicsCommandList` を直接書き換えることになり、それは presenter を通らない別の差し替え方を試すことになる。

---

## 5. 規模と危険度

- **規模: S（上限寄り）。** 器・窓・起動・検体・バルーンの組み立ては手本と窓口から写せる。新しく書くのは差し替えの system・当たり判定の標本点・分類と集計・較正・自動終了。案 B／C で画面取り込みを入れると、その組み立てが最大の塊になる。
- **危険度: 中。** 差し替えの部品と当たり判定の関数は既存で、挙動もコードから読める。未知は観測の側にある——WUC の暗黙の反映の単位、tick と DWM の合成の対応、取り込み画像での判別の安定性。いずれも「実験が答えを出す」種類であり、答えが出なければ「測れない」として README に書ける。

---

## 6. 議題（要件ディスカッションへ）

1. **基準の版は、走らせる前から崩れが出ると読める。** 素の再登録は古い子を残し、描画と当たり判定の重なり順が逆なので、混在と古い絵の残りが続く（§2.1）。この読みを実験で確かめること自体は価値があるが、合否基準の「直す」「違う」の線をどこに引くかが変わる。とくに版 (ii)（pilot が presenter の内部の entity を名前で探して消す）で崩れが 0 になった場合、それを「既存 crate を変えずに直せた＝直す」と見るか、「presenter に古い装着を片付ける口が無い＝本坑で present を変える必要がある（要件 5.5 の学び）」と見るか。
2. **要件 5.2 の 2 つの版（当たり判定を先に／絵を先に）は、公開の口では組めない。** 代わりに意味のある版は (ii) 除去あり・(iii) 別 id＋`Hide`・(iv) 差し替えを置く段（`Update`／`FrameFinalize`）である（§4）。5.2 の版の定義を差し替えるか、元の 2 版を「World を直接書き換えて組む」形で残すか。
3. **「フレーム」を何で数えるか。** wintf の tick（13 段を 1 回回る単位）で数えるか、DWM の合成（画面の更新）で数えるか。要件 3.1 の「途切れなく」と 3.4 の「同じフレーム」の意味がこれで決まる。tick で数えるなら当たり判定と揃えやすいが、画面の実物とはずれ得る。DWM で数えるなら画面の実物だが、当たり判定の状態との対応付けが要る。
4. **要件 3.2 を満たすには OS の画面取り込みが要る。** プロセスの中に「描画結果」を読む公開の手段は無い（§2.3）。取り込みを入れると規模が S の上限になり、窓が覆われていない・拡大率が分かっている、といった実行環境の条件も要る。取り込みを必須にするか（案 B／C）、World の状態で数えて取り込みは目視の補助に留めるか（案 A・要件 3.2 の緩和）。
5. **「実際に効いている当たり判定」は何を指すか。** wintf の `hit_test_in_window`（クリック透過の制御が使う判定関数）を標本点で問い合わせるのが自然だが、要件には書かれていない。OS の `WS_EX_TRANSPARENT` の付け外しはカーソル駆動で tick と揃わないので対象外にするか。collision の名前（`hit_region_client`）はバルーンでは常に空なので対象外でよいか。
6. **「どちらでもない」当たり判定の扱い。** 版 (iii) や `FrameFinalize` 置きでは、「絵は古いバルーン・当たり判定はどこも当たらない」という tick が生じ得る。これを「混在」に数えるか、別の数として出すか。また往復で窓の client 寸が 335×205 と 400×224 の間で変わるので、窓の矩形と絵の寸法が 1 tick 食い違う（絵が切れる・余白が出る）ことも起こり得る。これを崩れに含めるか。
7. **較正をどの段で行うか。** 分類の規則だけを、作った記録（「絵は A・判定は B」などの組）に当てるのか、実際の画面に崩れたフレームを出して取り込みから数えるのか。後者なら「古い絵の残り」は版 (i) で、「空」は `Hide` で、「混在」は pilot が `AlphaMaskResource` を差し替えて作れる見込みだが、較正のための仕掛けが増える。
8. **再登録は可視性の所有者を既定に戻し、窓寸の要求も初期化する。** 本番のバルーン窓は `External` なので、本坑で再登録を使うなら付け直しが要る。本先進坑では `CommandDriven` のままで実験できるが、この事実を README の学びとして残すか。
9. **wintf の兄弟の重なり順が、描画（先頭の子が上）と当たり判定（最後の子から調べる）で逆である。** 今は 1 つの窓に面が 1 枚なので表に出ていないが、差し替えに限らず「同じ窓に面を 2 枚重ねる」場面で絵と当たり判定が食い違う。本先進坑の範囲外の既存の食い違いであり、別途の起票（または直接修正候補への登記）を検討する価値がある。

---

## 7. 設計へ持ち越す調べ物

1. **WUC の暗黙の反映の単位。** 1 回の tick の中で行った「古い visual を外す／隠す」「新しい visual を作り描画面を描く」「木へ繋ぐ」が、DWM の 1 回の合成にまとめて載るか。載らなければ、同じ tick で揃えても画面では空の面や重なりが 1 回見え得る。
2. **despawn した entity の WUC visual がいつ木から外れるか。** 版 (ii) の成否に効く。wintf で despawn に反応して visual を外す処理（`cleanup_removed_entities_system`・`cleanup_surface_on_commandlist_removed` 周り）の段と時期。
3. **Desktop Duplication の組み立てと取りこぼしの検出。** `AcquireNextFrame` の `AccumulatedFrames`・`LastPresentTime` で「1 回も取りこぼしていない」を示せるか。`DwmGetCompositionTimingInfo`（`Win32_Graphics_Dwm` は workspace 既定で有効）で DWM の合成回数を並べて確かめられるか。
4. **取り込み画像での判別の規則。** 拡大率 k≠1 のときの寸法、窓の位置の割り出し（`WindowPos.position` か `GetWindowRect` か）、外形と縁の線で見るときの許容の幅。
5. **文字層の古い子が当たり判定に影響しないこと。** `Arrangement::default()` の寸法が 0 で `GlobalArrangement` の矩形も 0 になるかの確認。
6. **終了コードの返し方。** `WinApp::run` の終わり方と、観測完了での即終了・上限到達・失敗を区別して 0／0 以外を返す形。

---

## 8. 設計への申し送り（推奨の見立て・決定ではない）

- 器は手本 `crates/areka/examples/emo-present.rs` からバルーン窓 1 つ分だけを写すのが最短。依存は `crates/pilot/Cargo.toml` の `[dev-dependencies]` へ。
- 差し替えの system は **`Update` に置く**のが本番（`emo2_frame_system`）と揃い、1 tick のずれを持ち込まない。手本の `FrameFinalize` 置きは版 (iv) として比べる側に回すのが見通しがよい。
- 版は (i) 素の再登録（基準）・(ii) 古い子の除去あり・(iii) 別 id＋`Hide` の 3 つを最初から並べるのが、要件 5.2 の条件分岐を待たずに「直す／違う」の材料を揃える近道。ただし議題 1・2 の裁定次第。
- 観測は議題 3・4 の裁定で案 A／B／C が決まる。要件 3.2 を文字どおり守るなら案 B が最小。

---

## 9. 要件ディスカッションでの整理（2026-09-24）

### 9.1 査読の 2 周目（メインでの裏取り）

- §2.1 の「描画と当たり判定で兄弟の重なり順が逆」を実物で確かめた。`crates/wintf/src/ecs/graphics/systems/visual_sync.rs` の `visual_hierarchy_sync_system` のコメントは「最初の子が最上」、`crates/wintf/src/ecs/common/tree_iter.rs` の `DepthFirstReversePostOrder` の doc は「Children 配列の最後の要素（最前面）から走査」。読みは正しい。
- §2.1 の「`attach_target` は World に触れない」も `crates/areka-emo-present/src/presenter/hub.rs` の `attach_target` の定義（引数 `_world`・`self.targets.insert` だけ）で確かめた。

### 9.2 設計フェーズで決める事項（カテゴリ B）

1. 差し替えが揃った後に観測を続けるフレームの数（要件 3.1）と、既定の上限時間（要件 6.3）。どちらも README に書く。
2. 較正で崩れたフレームを実際の窓にどう作るか（要件 4・§6 議題 7 の「どの段で」は 4.1 の改訂で「実際の窓」に決着。作り方は設計）。候補: 残り＝古い子を残したまま新しい面を足す・空＝`Hide`・混在＝pilot が `AlphaMaskResource` を書き換える。
3. 観測の組み立て（§7 の調べ物 1〜6）。
4. README の学びに残す事実の候補: 再登録が可視性の所有者（`External` → 既定）と窓寸の要求を初期化すること（§6 議題 8）。
5. wintf の兄弟の重なり順の食い違い（§6 議題 9）は本先進坑の範囲外の既存の食い違い。README の学びに書き、起票は `/kiro-complete` の棚卸しで扱う（本 spec では直さない）。
