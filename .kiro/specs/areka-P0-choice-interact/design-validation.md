# 設計バリデーションレポート: areka-P0-choice-interact

- 実施日: 2026-07-24
- 対象: `.kiro/specs/areka-P0-choice-interact/design.md`（requirements.md / research.md §9 設計フェーズ追記を含む）
- レビュー方式: kiro-validate-design（design-review.md 準拠・非対話）＋既存コードへのスポット実証

## Review Summary

設計品質は極めて高い。最大の未確定点だった R-1（`HitTest::none()` バルーン窓へのポインタ到達経路）が「変更ゼロ・既存αマスク合流」として実コード証跡付きで解決されており、本レビューで**到達チェーンの全リンクを実コードに対して独立検証した結果、設計の主張はすべて事実と一致**した。donor 鏡写し（A-2×B-2×C-1）＋純関数中核の構成は steering の檻規律（判断分岐のみ檻・配線は再テストしない）と整合し、全 8 要件・全 AC のトレーサビリティが張られている。実装準備は整っている。

## スポット検証結果（設計の既存コード主張 vs 実測）

| 設計の主張 | 実測 | 判定 |
|---|---|---|
| バルーン surface entity＝`HitTest::alpha_mask()`＋`AlphaMaskResource`・offset (0,0) | `areka-emo-present/src/mount.rs:144-145`・`:65`/`:166` | 一致 |
| バルーン窓＝`HitTest::none()`・`BalloonWindowMarker`・clickthrough 登録は `GhostWindowMarker` 全窓 | `areka/src/placement/spawn.rs:174`・`:166`・`:303-304` | 一致 |
| `dispatch_pointer_events` が親チェーン（surface→窓）を Tunnel→Bubble 巡回・窓 entity で Bubble 受信可 | `wintf/src/ecs/pointer/dispatch/mod.rs:126-134`・`:162-188` | 一致 |
| `OnPointerExited` は dispatch されない（Moved/Pressed のみ）→ `PointerLeave` マーカー読みが必要 | dispatch/mod.rs で配信は Moved（:222）・Pressed（:232）のみ。`PointerLeave` は FrameFinalize の `clear_transient_pointer_state` で除去（`world/mod.rs:339-343`・`systems.rs:28-30`） | 一致（Input 段での leave 追随システム挿入は成立） |
| クリック＝`left_down` エッジ検出（1 dispatch のみ有効・dispatch 後クリア） | `dispatch/mod.rs:231`/`:245`。さらに `buffers.rs:174-191` は down_received を up より優先——同一フレーム内 down+up でも press は失われない | 一致 |
| 上流 3+1 API 実在（`ChoiceHitRow`/`inject_choice_hover`/`choice_hit_rows`/`choice_active`） | `areka-emo-text/src/actor.rs:150`/`:366`/`:389`/`:400` | 一致 |
| donor（`MouseWiring`/`attach_char_pointer_handlers`/DD-IE-10/DD-IE-12） | `areka/src/input_events/mod.rs:39`/`:232`/`:96`/`:217` | 一致 |
| `Emo2Wiring` に `runtime: Rc<RefCell<TextLayerRuntime>>` 実在・`presenter()` アクセサ同型・balloon `attach_target`・`ActorKey::from(scope.to_string())` | `areka/src/emo2_boot/frame.rs:183`・`:982-1001`・`:432`・`:460` | 一致 |
| main.rs の donor slot（clickthrough 登録・`attach_char_pointer_handlers` :585） | `areka/src/main.rs:567`・`:585` | 一致 |

R-1 裁定「spawn.rs／HitTest／clickthrough 改変ゼロ＝position-persist 衝突なし（rebase/merge 条項不発動）」は上記により裏付けられた。

## Critical Issues（いずれも非ブロッキング・タスクフェーズで解消可能）

🔴 **Critical Issue 1**: ハンドラ貫通の決定論テストが分解形のみで、R6.2 の字義（注入クリック→発行観測の一気通貫）を単一檻で満たしていない
**Concern**: Testing Strategy は `click_selection` 純関数檻＋「発行適用ステップへ合成 `ChoiceSelection` を通す」mpsc 檻に分解しており、`on_balloon_pointer_pressed` へ合成 `Phase<PointerState>` を与えて `ChoiceSelectionInbox.try_recv()` で観測する貫通テストが計画に無い。snapshot→純関数→適用の「糊」自体は実機目視のみが検証点になる。
**Impact**: 糊の結線ミス（借用順序・ordinal 展開・send 条件）が決定論檻を素通りし、実機サインオフまで検出が遅延する。steering「決定論的テスト網羅は必達」との緊張。
**Suggestion**: タスクフェーズで、テスト構築可能な `TextLayerRuntime`（frame.rs 檻が既に実演）へ選択肢行を決定論的に population できるか確認し、可能なら合成 PointerState→Inbox 観測の貫通檻を 1 本追加する。population が present（GPU）依存で不可能な場合のみ、現行の分解形を正当として設計の檻方針（配線は再テストしない）で閉じる。
**Traceability**: R6.2／R6.4（requirements.md Requirement 6）
**Evidence**: design.md「Testing Strategy」Integration Tests 2・3／Requirements Traceability 6.1-6.3 行

🔴 **Critical Issue 2**: `clear_balloon_hover_on_leave` のスケジュール登録は配線存在檻の対象外
**Concern**: R6.6 の配線存在檻はバルーン窓への `OnPointerMoved`／`OnPointerPressed` コンポーネント存在を assert するが、leave 追随システムは main.rs のスケジュール登録（Input・dispatch 後）であり、登録漏れを検出する檻が計画に無い（leave 対象選別ロジックの bare World 檻はある）。
**Impact**: 登録漏れ時、高速離脱で hover が残置し R1.3／R7.1（目視追従）が静かに毀損する。検出は実機目視のみ。
**Suggestion**: 配線存在檻に「Input スケジュールへの `clear_balloon_hover_on_leave` 登録存在」の assert（Schedules 資源の system 列挙）を 1 本足すか、少なくとも tasks.md の該当タスクへ登録確認を明記する。
**Traceability**: R1.3／R3.4／R6.6
**Evidence**: design.md「clear_balloon_hover_on_leave」Implementation Notes・「Integration Tests」1／5

（3 件目に相当する重大懸念なし。バルーン面α不透明性への依存・M1 Inbox 滞留・hover_inject 併用は設計が Revalidation Triggers／Risks で明示済みかつ Low で妥当。）

## Design Strengths

1. **R-1〜R-4 の実測解決と Revalidation Triggers の明文化**: 最重要リスク（ポインタ到達）を推測でなく wintf/emo-present の一次コードで解き、到達契約の前提（αマスク供給・mount offset (0,0)・出力順）を再検証トリガとして固定した。本レビューの独立実証で全リンクが正であることを確認済み——spawn.rs 不改変の帰結により position-persist との衝突条項も正しく不発動。
2. **純関数中核×donor 鏡写しの檻適合構造**: 全判断分岐（hit／hover 遷移／click 確定／stale 棄却）を World・runtime 借用の外の純関数 3 本へ集約し、steering「檻に入れるのは判断分岐のみ」「実窓・sleep 不要」に構造レベルで適合。`ChoiceSelection` から ordinal を排し `ResolveChoice{id}` と整合させた契約設計（DD-CI-4）も下流結合を最小化している。

## Final Assessment

**Decision: GO**

**Rationale**: 既存アーキテクチャとの整合（donor 同型・依存方向・スレッド親和・素通し規約）に齟齬がなく、全要件がコンポーネント／檻へ追跡され、最大リスクだった到達経路は実コード検証で裏が取れた。指摘 2 件はいずれもテスト計画の補強であり、設計の構造変更を要さずタスクフェーズで吸収できる（受容可能リスク）。

**Next Steps**:
1. 設計ディスカッション（kiro-design-discussion）で Issue 1・2 の扱い（貫通檻の可否確認／登録存在檻の追加）を裁定し、必要なら tasks.md へ反映。
2. `/kiro-spec-tasks areka-P0-choice-interact` で実装タスクを生成（Issue の裁定結果をタスクへ織り込む）。
