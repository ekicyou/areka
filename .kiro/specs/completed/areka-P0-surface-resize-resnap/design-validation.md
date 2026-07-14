# 設計検証レポート: areka-P0-surface-resize-resnap

> 実施日: 2026-07-13 / スキル: kiro-validate-design（非対話・自動判定）
> 対象: 確定済み requirements.md（承認済）＋ design.md（生成済・未承認）＋ research.md ＋ steering
> 実コード照合: follow.rs / resolver.rs / config.rs / spawn.rs / target_map.rs / emo2_boot/frame.rs / areka-emo-present / wintf graphics・window を実読して設計の再利用主張を追認

---

## 設計レビュー要約

本設計は「シェル座標系（アンカー辺）→ ウィンドウ座標系（サーフェス寸法）の変換 T の恒常維持」という要件の幹を、既存資産（`BottomSnapPolicy`／単一位置ライター／`MonitorSnapshot`／`follow_balloon`／`text_slot_view`／`GhostWindows`・`target_map`）の合成へ忠実に落とし込んでおり、依存方向・物理 px 単一通貨・log-first・本番ゴースト先行の各規律を厳守している。設計が名指しした挿入点・署名（`BottomSnapPolicy.resolve`／`enqueue_window_move` の `SWP_NOSIZE` 固定／`SetWindowPosCommand.width/height`／`text_slot_view().surface_size()`／`run_drain_phase` の同一 World シーム／`Alignment=Bottom/Free/Seam`）はすべて実コードに存在し、char 窓は emo-present 自前の `SwapChainPresenter`＋`VisualMount`（`ShowSurface` で既に新寸へ resize 済）で描画されるため、本 spec は「HWND を合成寸へ追随させる」ことに徹すれば整合する——設計の境界設定は正しい。トレーサビリティ表・決定論檻・実 DPI 目視受け入れも網羅されており、実装着手に足る品質にある。**結論は GO**。以下 3 点は設計ディスカッションで詰めるべき refinement（いずれも NO-GO の阻却事由ではない）。

---

## クリティカルイシュー（≤3）

### 🔴 Critical Issue 1: 最大リスク（move+resize echo による窓振動・バルーンのクリック死）に対応する検証ゲートが受け入れ基準に無い

**Concern**: 設計自身が最大リスクと認めるのは、単一ライター bypass が move 専用（`SWP_NOSIZE` 固定）で作り込まれてきた経路へ size 変化を足すことで生じる `WM_WINDOWPOSCHANGED` echo の二重反映＝**窓振動・バルーンのクリック死という既発の実機ブロッカ面**（§10 リスク・§4-1・follow.rs doc の警告）。しかし決定論檻は偽 HWND ゆえ実 echo を観測できず、R5 の受け入れ基準（5.1）は「切替後もアンカー辺が保たれる（宙に浮かない）」だけを目視条件にしており、**「振動しない」「resize 後もバルーンがクリック可能」という退行面が、どの受け入れ基準・E2E テストにも明示されていない**。最も壊れやすい面が観測ゲートを持たない。

**Impact**: dpi=96 自己整合が欠陥を隠した window-placement リジェクトの教訓が示すとおり、退行面を名指ししない受け入れは「アンカーは合っているが振動する／バルーンが死ぬ」状態を GO と誤認しうる。

**Suggestion**: R5 の実 DPI 手動受け入れに「切替直後に窓が振動しないこと」「resize 後もバルーン透過ヒットが生きていること」を明示の観測項目として追加し、`enqueue_window_set_pos(.., Some)` が `WindowPos.size` を bypass ミラーしつつ二重の `SetWindowPos` を発行しない echo 抑止不変を（可能な範囲で）観測に落とす。設計の Testing Strategy E2E と §Monitoring に退行面の証跡取得を明記する。

**Traceability**: Req1.5（単一ライター・bypass 新設なし）・Req1.7（一度書き・振動なし）・Req5.1/5.2（実 DPI 目視受け入れ）
**Evidence**: design.md §Error Handling／§10 リスクと緩和（追補）／§Testing Strategy「E2E / 手動受け入れ」（5.1 が anchor 維持のみを条件化）

### 🔴 Critical Issue 2: アンカー表現の二重化（`bottom_snap` の「導出 or 併存」・`BottomSnap` marker の退役）が未確定で、単一の真実源が曖昧

**Concern**: Req1.6 は「座標系変換の実装を二重化しない」を要求するが、設計は `ScopePlacement` へ `pub anchor: Anchor` を足す一方で既存 `bottom_snap: bool` を「`anchor` から**導出 or 併存**」と両論併記で残している（§File Structure）。実コードでは `bottom_snap` は resolver.rs:200 で `matches!(alignment, Bottom | Seam(_))` として全 `Seam`（top/left/right/未知）を true へ畳んでおり、spawn.rs は `DragConfig.move_window = !p.bottom_snap`（199-200）と `BottomSnap` marker 付与（218）、follow.rs は `on_char_drag` が `world.get::<BottomSnap>()`（163）で分岐、spawn テスト（594）は `BottomSnap` 存在を assert している。`Anchored(Anchor)` を新設しつつ `bottom_snap`／`BottomSnap` を併存させると、drag（Free 判定）と resize（射影分岐）で真実源が二つになる。

**Impact**: 二重表現は top/left/right を「bottom へ畳む旧経路」と「固有アンカーで射影する新経路」に分岐させ、どちらが勝つかが呼び出し箇所依存になる。R1.6 が排除しようとした二重化そのものを招き、既存 spawn テストの資産（`BottomSnap` 前提）と衝突する。

**Suggestion**: `Anchored(Anchor)` を単一の真実源に確定し、`bottom_snap`／`BottomSnap` marker は退役（`Anchored` から導出する薄いヘルパへ吸収 or 全廃）を明記する。resolver は `anchor` のみを運び、spawn の `move_window` は `matches!(anchor, Anchor::Free)`、on_char_drag/on_char_drag_end は `Anchored` 読取りへ一本化。併せて `BottomSnap` 存在を検証する既存 spawn テスト（spawn.rs:580-604）の更新方針（`Anchored(Bottom)` 検証へ差し替え）を設計へ書き込む。

**Traceability**: Req1.6（同一 T・二重化しない）・Req4.2（解決済みアンカーの消費表現）
**Evidence**: design.md §File Structure（resolver.rs「`bottom_snap: bool` は `anchor` から導出 or 併存」）／§Anchored（Component）／実コード resolver.rs:200・spawn.rs:199-218・follow.rs:163

### 🔴 Critical Issue 3: 新規に有効化される top/left/right の runtime ドラッグ経路が檻で明示網羅されていない

**Concern**: 現状 top/left/right は `Seam(_)` として bottom へ畳まれ、ドラッグ実挙動は「Y を下端へ釘付け＋警告」（config.rs:21・window-placement DD9）。本設計は `Anchored`＋`project_anchor` へ切替えることで、top/left/right のドラッグ／resize を**固有アンカー辺固定（例 `left`: X=`wa.left`・Y 保持＝自由軸は上下）へ初めて挙動変更する**。ところが Testing Strategy の Integration Test #8「drag 統一」は `Bottom`（Y 釘付け）と `Free`（wndproc 委譲）のみを名指しし、top/left/right のドラッグ自由軸方向・`DragConstraint` の runtime 檻が列挙されていない。純粋 `project_anchor` は全 5 アンカー網羅（Unit #1）だが、on_char_drag が `Anchored` 経由でその射影を non-Free 全アンカーに正しく配線するかの観測は Bottom 一件に留まる。

**Impact**: `project_anchor` は純粋に正しくても、runtime の drag 配線（`Anchored` 読取り→射影適用→`DragConstraint` との整合）が top/left/right で未観測だと、「純関数は緑・実ドラッグは崩れる」隙が残る。ただしプロジェクトの検証方針（判断分岐＝Free-vs-non-Free の二値のみ檻・射影は檻済み純関数へ委譲＝`test-only-decision-branches` メモリ）に照らせば、Bottom 一件で non-Free 枝を代表させる読み方も成立しうる——この線引きの是非を確定すべき。

**Suggestion**: 設計ディスカッションで「non-Free ドラッグ枝は Bottom 一件で代表させる（射影は project_anchor 純関数檻へ委譲）」と明記して意図的縮約を宣言するか、あるいは Integration Test #8 に left または right の runtime ドラッグ檻を最低一件追加して自由軸方向（例 `left` は上下自由・X=`wa.left` 固定）を固定するか、いずれかを確定する。

**Traceability**: Req1.6（drag と resize 同一 T）・Req2.2–2.4（top/left/right 射影）・Req5.4（決定論網羅）
**Evidence**: design.md §Testing Strategy「Integration Tests」#8（Bottom/Free のみ）／§project_anchor Validation（純粋は全アンカー網羅）／実コード follow.rs:163・config.rs:21

---

## 設計の強み

1. **実コードで裏取りされた再利用設計**: 名指しした全挿入点・署名（`BottomSnapPolicy.resolve(raw,size,snapshot)`／`enqueue_window_move` の `SWP_NOSIZE` 固定／`SetWindowPosCommand.width/height`／`text_slot_view().surface_size()`／`run_drain_phase` の同一 `&mut World`＋presenter シーム／`target_map` の `2*scope`／`GhostWindows`）が実在を確認できた。frame.rs は既に balloon 経路で「apply 直後に同一フレームで `text_slot_view` を読む」前例（409 行）を持ち、DD-2 の「同一 World 直接呼び・チャネル不要」は既存パターンの延長として自然に成立する。新規 crates.io 依存・新規通信フレームワーク・tokio をゼロに保った制約遵守も確実。

2. **境界と依存方向の規律が明晰**: 検知（emo-present read-only）→判定（純粋 diff/射影）→反映（placement 単一ライター）を `frame.rs` が下流統合層として結び、emo-present は placement を一切知らない（依存を汚さない）。char 窓の描画は emo-present 自前 `SwapChainPresenter`＋`VisualMount`（`ShowSurface` で新寸へ resize 済）が所有するため、本 spec が「HWND を合成寸へ追随」に徹する境界設定は正しく、cascade 解決・cue routing・合成中身を非所有とする Out of Boundary の線引きも一貫している。`Anchor` を純粋 `resolver`、射影 `project_anchor`／`Anchored` を wintf/bevy 層 `follow` に配置する層割りも U5 純粋檻を保つ。

---

## 最終評価

### 判定: **GO**

**Rationale**: 要件の幹（変換 T の恒常維持）を既存資産の合成へ忠実に写像し、依存方向・物理 px 単一通貨・単一ライター・log-first・本番ゴースト先行の各規律を守り、設計の全再利用主張が実コードで追認できた。アーキテクチャの根本的不整合・スコープ肥大・失敗リスクは無く、決定論檻＋実 DPI 目視受け入れの実装経路が明確。上記 3 イシューはいずれも受け入れ観測の明示化・単一真実源の確定・檻の縮約宣言という refinement であり、設計ディスカッションで解消可能で着手を妨げない。

### 次のステップ

1. 設計ディスカッション（`/kiro-design-discussion areka-P0-surface-resize-resnap`）で Issue 1〜3 を詰める:
   - Issue 1: R5 受け入れへ「振動なし」「resize 後もバルーンがクリック可能」を明示観測項目として追加。
   - Issue 2: `Anchored(Anchor)` を単一真実源に確定し `bottom_snap`／`BottomSnap` marker の退役方針と既存 spawn テスト更新を明記。
   - Issue 3: non-Free ドラッグ枝の檻を Bottom 代表で縮約するか left/right 一件追加するかを確定。
2. 反映後、`/kiro-spec-tasks areka-P0-surface-resize-resnap` でタスク生成へ進む。

---

## 補足（設計ディスカッションへの論点・非ブロッキング）

- **`WindowPos.size` の client/window 寸等価**: baseline diff（`shown_size != WindowPos.size`）は、borderless GPU 合成窓（`WS_EX_NOREDIRECTIONBITMAP`・フレーム無）ゆえ client==window で成立する。設計にこの等価前提を一行明記しておくと後続 M-dual/装飾窓での落とし穴を防げる（現時点はスコープ内で正しい）。
- **Free 窓の resize 時 position 再表明**: `resize_window_to(Free)` は `project_anchor` identity で現 position を据え置くが、`enqueue_window_set_pos` は常に x,y を渡す（`SWP_NOMOVE` を立てない）。`\s` 切替時は非ドラッグ中ゆえ無害だが、将来ドラッグ中 resize が起きうるなら `SWP_NOMOVE` 条件化の余地を残すと堅い（現要件では非該当）。
- **`anchor_changed_system` の producer 不在**: Req1.4 の runtime producer（seriko `\![set,alignmenttodesktop]` routing）は非所有かつ未実装ゆえ、当面は spawn 時付与のみが実 producer。`Changed<Anchored>` の初回発火はべき等 skip で吸収する設計の扱いは妥当（テストは `Anchored` 直接 mutate で駆動＝正しい consumer 契約檻）。
