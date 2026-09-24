# 設計レビュー: pilot-balloon-asset-swap

- レビュー日: 2026-09-24
- 対象: `design.md`（`spec.json.phase = design-generated`）・`requirements.md`（承認済み）・`research.md` §9（開発者の裁定）・§10（設計フェーズの決定）・`.kiro/steering/two-tunnel.md`
- 進め方: `design.md` の file:line／API の主張を実物のコードと `windows` crate（レジストリの実体）で 1 つずつ裏取りし、検体 2 つの PNG を実際に開いて標本点の規則を数字で確かめた。非対話（質問はしない）。

## レビュー要約

構造は正しい。差し替えの 3 版 × 2 段・`\b[ID]` 相当の面の切り替え・実際の画面の取り込み（Desktop Duplication）・実際の窓で作る較正 5 項・有界な自動終了・README 3 幕という骨組みは、開発者の裁定（§9.3）と要件 2〜7 を漏れなく写しており、変更は `crates/pilot/` の下に閉じている。設計が引く API・段の名前・フックの存在はすべて実物と一致した。

ただし、設計が自分で決めた「標本点の規則」を検体の実物に当てると、面の切り替えの判別対 `(A0, A2)` が規則の側で「見分けられない」に落ち、較正 `calib-size` が必ず不合格になって走行全体が無効になる（下記 課題 1・実測で確認）。これは設計の局所的な直しで消せる。加えて、突き合わせの規則が「合成器の遅れ」を混在 1 として数に出し得ることを設計自身が認めているのに、その内訳が判定の材料（info・README の表）に出ない（課題 2）。

## 裏取りの結果（設計の主張 → 実物）

| 設計の主張 | 実物 | 判定 |
|---|---|---|
| `VisualGraphics` の `on_remove` フックが despawn 時に親の `ContainerVisual.Children().Remove(visual)` を呼ぶ | `crates/wintf/src/ecs/graphics/components.rs` の `#[component(on_remove = on_visual_graphics_remove)]` と `on_visual_graphics_remove` の本体（`parent.cast::<ContainerVisual>()…Children()…Remove(visual)`） | 一致 |
| tick は 13 段 `Input → … → FrameFinalize` | `crates/wintf/src/ecs/world/mod.rs` の `try_tick_world`（`try_run_schedule` を 13 本固定順で呼ぶ）と `apply_window_pos_changes` の `UISetup` 登録 | 一致 |
| `hit_test_in_window(&World, Entity, PhysicalPoint) -> Option<Entity>`・`WindowPos.position` で screen 座標へ・`Visual.is_visible` は α マスク判定に効かない | `crates/wintf/src/ecs/layout/hit_test/mod.rs` の `hit_test_in_window` と `hit_test_entity`（`HitTest.mode`・`GlobalArrangement.bounds`・`alpha_mask_hit` だけを見る） | 一致 |
| `GlobalArrangement` は不可視でも計算される（較正「混在」の前提） | `crates/wintf/src/ecs/layout/systems/arrangement_systems.rs` の `propagate_global_arrangements` に可視性の条件は無い | 一致 |
| `windows` 0.62.2 に feature `Win32_Graphics_Dxgi`・`IDXGIOutput1::DuplicateOutput`・`AcquireNextFrame`・`ReleaseFrame`・`DXGI_OUTDUPL_FRAME_INFO { LastPresentTime, AccumulatedFrames, .. }` | `Cargo.lock` の `windows 0.62.2`・レジストリ `windows-0.62.2/Cargo.toml` の `Win32_Graphics_Dxgi = ["Win32_Graphics"]`・`src/Windows/Win32/Graphics/Dxgi/mod.rs` の各定義 | 一致 |
| `Win32_Graphics_Dxgi` は workspace 既定に無く pilot で足す・`Direct3D11`／`Performance`（QPC）／`WindowsAndMessaging`（`GetWindowRect`・`GetWindow`）は既定 | ルート `Cargo.toml` の `[workspace.dependencies.windows].features`（`Win32_Graphics_Dxgi_Common` のみ・`Win32_Graphics_Direct3D11`・`Win32_System_Performance`・`Win32_UI_WindowsAndMessaging` あり）。`areka-emo-text` だけが `Win32_Graphics_Dxgi` を上乗せしている | 一致 |
| `attach_target(_world, TargetId, Entity, EmoWorld, AtlasTable, author_dpi)`・`apply(PresentCommand)`・`take_pending_resize`・`applied_scale`・`current_surface_id`・`target_physical_size`・`PresentCommand::{ShowSurface, Hide}` | `crates/areka-emo-present/src/presenter/{hub,refresh,read}.rs`・`command.rs` | 一致 |
| 子の名前 `emo-surface`／`emo-text-layer-slot`・`Hide` は `Visual::set_visible(false)`＋両 entity `HitTest::none()`・mount の `Arrangement.offset` は 0（左上基準） | `crates/areka-emo-present/src/mount.rs`（`Name::new(...)`・`set_visible` の doc・「offset は 0 でなければならない」） | 一致 |
| `WinApp::new()` が `PER_MONITOR_AWARE_V2`・`ExitPolicy` 既定 `OnLastWindowClose` | `crates/wintf/src/runtime/mod.rs` | 一致 |
| `Composer::compose(&EmoWorld, &AtlasTable, surface_id, &BindSet, &PatternState) -> ComposedSurface`（premultiplied BGRA） | `crates/areka-emo-compose/src/lib.rs`・`composed.rs` | 一致 |
| `SampleRoot::acquire("StayseeBalloon")`・`.balloon("emo2-kakukaku")`・`build_balloon_target(dir, &decoder, 0)` | `crates/sample-ghost-kit/src/lib.rs`・`crates/areka-emo-present/src/balloon.rs` | 一致 |
| 変更は `crates/pilot/` の下だけ・他 crate に `pilot` を足さない・S 規模 | Modified Files は `crates/pilot/Cargo.toml` のみ。新規 4 ファイル＋README。前例 `pilot-clickthrough-alpha-toggle`（1,006 行）と同程度 | 適合 |

## 重大な課題（最大 3）

### 課題 1: 判別対 `(A0, A2)` が設計自身の標本点の規則で「見分けられない」になり、較正 `calib-size` が必ず不合格になる

- **問題**: 設計 §Observer は、判別対の共通範囲（幅・高さの小さい方）を 8×8 の升に切り、「P だけ不透明」「Q だけ不透明」「両方不透明で色差 ≥ 48」の 3 集合それぞれで **16 点未満なら「見分けられない」＝その対の観測はすべて「測れない」**と定めている。検体の実物（`vendors/sample_ghost/StayseeBalloon.nar` の `balloons0.png` 335×205 と `balloons2.png` 335×395）を開いて共通範囲 335×205 で数えると、**「A0 だけ不透明」は 108 画素しか無く、8×8 の升に割ると 3 升程度**（右端 x≥315 の帯・行 81〜90 と 183〜203 に集中）。「A2 だけ不透明」は 5,518 画素（18 升）、色差 ≥ 48 は 3,415 画素（21 升）ある。つまり A2 の上 205 行は A0 をほぼ含む形で、**A0 だけの点がほとんど無い**。規則どおりなら `(A0, A2)` は起動時に「見分けられない」となる。参考: `(A0, B0)`（kakukaku 400×224）は 26／21／38 升で規則を満たす。
- **影響**: 面の切り替え（要件 2.6・2.7）の 2 観測が全部「測れない」になるだけでなく、`calib-size`（要件 4.5・A のまま面 2 を出して窓寸を合わせない）も測れず「大きさの食い違い ≥ 1」を満たせないため、**要件 4.7 により本番の数がすべて「無効」・終了コード 3** になる。また設計 §Observer は「対は `(A0, B0)`（資産の差し替え・**較正**）と `(A0, A2)`（面の切り替え）」と書くので、`calib-size` を `(A0, B0)` で判別すると A2 の絵は P とも Q とも一致せず「測れない」に落ちる（対の指定が食い違っている）。
- **提案**:
  1. 標本点の規則を「片方が他方を含む対」に耐える形へ改める。例: 3 集合のうち **「Q だけ」と「両方（色差 ≥ 48）」の 2 集合が ≥ 16 なら可**とし、P／Q の判別を `P: b ≤ 0.1 ∧ ab_p ≥ 0.9`・`Q: b ≥ 0.9 ∧ ab_q ≥ 0.9` で行う（「P だけ」集合は在れば使い、無ければ分母から外す）。共通範囲を「大きい方の外形」に広げて範囲外を透明扱いにする案は、`calib-size` では窓 335×205 の外の点が分母から外れるので単独では足りない。
  2. `calib-size` と `face-switch` の判別対を **`(A0, A2)`・`to = A2`** と明記する（§SwapDriver の較正の表と §Observer の対の列挙を揃える）。
  3. 起動時に導出した点の数（集合ごと・対ごと）を `info!` だけでなく README の検証結果にも書き、「見分けられない」の判定を実測に基づいて説明できるようにする。
- **Traceability**: 要件 2.6・2.7・4.5・4.7・3.6
- **Evidence**: design.md §Observer「標本点（`Signature`）は判別対 `(P, Q)` ごとに…どれかの集合が 16 点未満なら」・§SwapDriver「`CalibSize`（4.5）」・Key Decisions 5・§Testing Strategy

### 課題 2: 「合成器の遅れ」による混在 1 が版に依らず出得るのに、判定に使う数（info・README）でそれを見分けられない

- **問題**: 突き合わせの規則（取り込んだ画面更新 T 以前に終わった最新の tick と組にする）は要件 3.2 の定めどおりで、**偽の混在ではない**——tick が終わって World（当たり判定）が新しくなった後、DWM がその絵を出すまでの間は、利用者から見ても「絵は古い・クリックは新しいマスク」である。しかし設計 §Observer Risks と research §10.2 が認めるとおり、これは版に依らず（本命の版でも `\b[ID]` の面の切り替えでも）混在 1 フレームとして数に出得る。要件の合否基準は「直す＝本命の版で崩れが **0**」なので、**合成器の遅れが 1 出るだけで「直す」が構造的に到達不能になり「違う」へ倒れる**。設計はこの内訳（`picture=from, hit=to`）と `present_qpc − ended_qpc` を `debug!` に出すに留め、集計ログ（info）と README の表には「混在」の合計しか出ない。
- **影響**: 開発者が README の数だけを見て go／違う／直す を判定する（要件 7.5・7.6）ときに、「差し替えに固有の混在」と「観測の限界（合成器の遅れ）による混在」を分けられない。先進坑の唯一の成果物である知見の質に直結する。
- **提案**:
  1. `Counts` に **「揃う前の混在」**（揃ったフレームより前で、絵 = `from` ∧ 当たり判定 = `to` の組）を内訳として持ち、`log_observation` と README の表で「混在（うち揃う前）」の形で出す。要件 3.4 の 4 種と重複計上の枠内であり、語彙の追加ではない。
  2. `face-switch@update`（本番で既に通っている経路）の「揃う前の混在」を**床**として README に並べ、見立ての欄に「本命の版の揃う前の混在が face-switch と同数なら差し替え固有ではない」と書く規則を design に明記する。
  3. `calib-static`（4.6）はこの遅れを較正できない（差し替えが無い）ので、README の「分からないこと」に「揃う前の混在 1 は較正で 0 と確かめられない」と書く。
- **Traceability**: 要件 3.2・3.4・5.9・7.4・7.5、research §9.3（案 B の裁定）
- **Evidence**: design.md Key Decisions 4「この規則で…合成器の遅れが 1 フレームの混在として数に現れ得ることは…」・§Observer Implementation Notes「Risks: 合成器の遅れ…」・§README に残す学びの候補

### 課題 3: 当たり判定の 4 つ目の値「両方」は要件 3.3・3.4 の語彙に無く、要件側の追記が要る

- **問題**: 要件 3.3 は当たり判定を「一方・他方・どちらでもない」の 3 値で列挙し、3.4 の「混在」も「絵と当たり判定が別のバルーンのもの・片方だけがどちらでもない・絵に両方が同時に見えている」の 3 形だけを挙げる。設計 Key Decisions 9 は `HitClass::Both`（古いマスクと新しいマスクが同時に効く＝基準の版で必ず起きる状態）を足し、`Both` を含む組をすべて混在に数える。これは「新しい絵と同じ当たり判定ではない」状態を 0 と数えないための正しい判断だが、要件の文面では 3.4 のどの形にも当たらず、要件と設計の語彙が食い違う。
- **影響**: 作業は変わらない（設計の規則で数える）。ただし README が「混在」の定義を要件の文面で写すと `Both` の混在が説明できず、後で本坑の design が README を参照するときに解釈の余地が残る。
- **提案**: 設計ディスカッションで要件 3.3 の当たり判定の列挙に「両方」を足し、3.4 の混在に「または当たり判定に両方のバルーンのマスクが同時に効いている」を足す（研究記録 §10.4 の Follow-up どおり）。design 側は現状のままでよい。
- **Traceability**: 要件 3.3・3.4
- **Evidence**: design.md Key Decisions 9・§Observer「当たり判定の判別…`Both`: 両方 ≥ 0.9」・research §10.4「Decision: 当たり判定の「両方」を 4 つ目の値として持ち…」

## 重大ではない注意（記録のみ）

- **tick の門**: wintf には「変わった」旗が無い画面更新で tick を省略する門（`crates/wintf/src/ecs/world/tick_gate.rs`・心拍は 30 画面更新に 1 回）があるが、`EcsWorld` の既定は `tick_gate_enabled: false` で、`AREKA_TICK_GATE` を読むのは `crates/areka/src/tick_gate_config.rs`（本体だけ）。pilot の `WinApp` では門は切れたままなので、設計の「30 tick 後に閉じる」「180 tick で未完」は画面更新ごとの tick を前提にしてよい。将来 wintf 側で既定が変わったときのために、Revalidation Triggers に「tick の門の既定」を 1 行足しておくとよい。
- **ずれの規則の健全性（false 0 の有無）**: 観測の窓の中で World が変わる時点は要求 tick（と `calib-stale` の追い打ち）だけなので、「T 以前に終わった最新の tick」が 1 つ前の状態の当たり判定と組になることは無い。`FrameFinalize` の版で新しい子に `GlobalArrangement` が無い tick は「絵は古い・当たり判定は古い（Reattach）」または「空・どちらでもない（RemoveThenAttach）」と正しく数えられる。偽の 0 を生む経路は見つからなかった。
- **`Reset` 後の presenter の残り登録**: `AttachNewHideOld` と `calib-mixed`／`calib-stale` で使った `TargetId(2)`／`next_id` の登録は `despawn_mounts` の後も presenter の表に残る（mount の entity は消えている）。以後その id へ `apply` しない限り害は無いが、README の学びに「再登録の口しか無いので古い id の登録は消せない」と 1 行添える価値がある。
- **標本点の色の一致（許容 12）**: `(A0, B0)` の共通範囲は「両方不透明で色差 ≥ 48」が 38 升あり十分。`(A0, A2)` は課題 1 の直しの後で「両方」21 升・「Q だけ」18 升が使える。

## 設計の強み

1. **段の違いを版の差にした点**（Key Decisions 2）: 同じ system を `Update` と `FrameFinalize` に登録し台本が段を選ぶので、「1 フレーム遅らせる解」を直し方ではなく比較の版として、コードの複製なしに並べられる。§9.3 の裁定と記憶「1 フレーム遅らせる解は取らない」の両方を満たす形になっている。
2. **較正が既存の公開部品だけで実際の窓に崩れを作る点**（Key Decisions 6・§SwapDriver）: `Hide`・`HitTest` の付け替え・`Visual::set_visible`・`take_pending_resize` を捨てる・窓を 1px 動かす、の 5 項がどれも wintf／present の公開 API で成り立ち（`hit_test_entity` が可視性を見ないことを実物で確認）、`AlphaMask` を自前で組む必要が無い。要件 4.1 の「作った記録を規則に当てるだけの較正で代えない」を守っている。

## 最終判断

**GO（条件付き）**

- **理由**: 骨組み・依存の向き・段・API・フックの存在は実物と一致し、裁定 §9.3 と要件 2〜7 を漏れなく写している。課題 1 は設計の局所的な規則（標本点の集合の条件と `calib-size` の対の指定）の直しで消え、課題 2 は集計の内訳を 1 つ足すだけ、課題 3 は要件の文面の追記で済む。いずれも設計ディスカッションの範囲で解ける。
- **条件**: 課題 1 を design.md に反映してからタスク生成に進むこと。反映しないまま実装すると、検体の実物により `calib-size` が必ず不合格となり走行全体が「無効」になる。
- **次の手**: `/kiro-design-discussion pilot-balloon-asset-swap` で課題 1〜3 を裁定し design.md（と要件 3.3・3.4 の語彙）を改めた後、`/kiro-spec-tasks pilot-balloon-asset-swap`。
