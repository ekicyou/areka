# 設計レビュー: areka-P0-collision-geometry

> 実施: 2026-07-17 / `/kiro-validate-design`（非対話・レポートはディスクへ永続化）
> 入力: requirements.md（R1–R7 確定）・design.md・research.md・`.kiro/steering/`（product/tech/structure/roadmap/logging/focus）
> 方式: design.md の**負荷を担う主張を実コードへ全数突合**した上での GO/NO-GO 裁定

## Design Review Summary

純粋コア＋薄い合成（Functional Core / Imperative Shell）への三分割は要件・steering・crate 憲章のいずれとも整合し、正典が沈黙する4点（含端・Ref4 非該当値・ローカル座標空間・透明画素）を「SILENT の明示＋設計判断＋単一変更点（再検証トリガ）」で決着させた確定表は、この種の設計文書として例外的な水準にある。**本レビューで実コードへ突合した design の主張は全て CONFIRMED**（下記 Strengths 参照）＝机上の整合でなく実測に立脚している。批判は設計の中核（純関数・additive 読み口・`HitRegion` 契約）には向かわず、**3件すべてが「7.3 probe の証拠力の到達範囲」と「本番呼び手への結線」に集中**する。いずれも実装着手を阻む性質ではなく設計ディスカッションで詰めるべき論点ゆえ **GO**。

## Critical Issues

### 🔴 Critical Issue 1: リゾルバの「本番呼び手からの到達性」が未実証（R4 の窓口性そのもの）

**Concern**: `resolve_hit_region(presenter: &EmoPresenter, scope, x, y)` は presenter を**引数で受ける**が、本番の第一消費者＝wintf のポインタハンドラは `fn(world: &mut World, sender, entity, ev: &Phase<PointerState>) -> bool`（`crates/wintf/src/ecs/pointer/dispatch/mod.rs:68-72`）＝**`&mut World` しか持たない**。一方 presenter は `Emo2Wiring`（**NonSend resource**）に内包され、`emo2_frame_system` が毎フレーム **World から remove→3フェーズ→insert** する（`crates/areka/src/emo2_boot/frame.rs:637-659`・`:171`）。つまり「ハンドラがどう `&EmoPresenter` を得るか」「remove 窓の間に配送されたら `Emo2Wiring` は不在ではないか」が design のどこにも無い。C-8 は「配線を frame.rs へ寄せる／関数ポインタ・trait 注入で受ける」と**回避形の存在だけ**を W2 へ申し送るが、**リゾルバの署名がイベント源から呼べること自体は検証されていない**。

**Impact**: R4 の Objective は「input-events が…**同期で得られる単一の窓口**」であり、呼び手から到達できない窓口は要件を満たさない。しかも 7.3 probe は donor（`emo-present.rs`）方式で **presenter を example が直接所有**するため、probe が PASS しても**本番呼び手が呼べる証拠は1つも得られない**。W2 が「呼べない」に到達するのは実装着手後＝ウェーブ直列の意味が薄れる。

**Suggestion**: design に**呼び手側アクセスシムを1つ確定**する（(a) `Emo2Wiring` から presenter を借りるヘルパを `emo2_boot` 側に置く／(b) ハンドラは点を積むだけにして解決を frame 駆動の poll へ倒す／(c) 関数ポインタ注入）。選んだ形は `hit_region.rs` の `crate::` フリー規律と両立するかを併記し、**probe が同じシムを通る**ようにすれば Issue 2 も同時に解消する。ポインタ配送スケジュールと `emo2_frame_system` の remove 窓が重ならないことも明記されたい。

**Traceability**: R4.1 / R4.2（UI スレッド同期の単一窓口）・R5.4（正本性＝消費側が握る1点）
**Evidence**: design.md「Components and Interfaces / Resolver / Service Interface」・「Coordination Notes C-8」・「Probe / Batch 契約」

### 🔴 Critical Issue 2: probe が本番配線でなく「並行宇宙」を再構築している（steering 原則との緊張）

**Concern**: probe は本番の表示配線を通らない。(a) presenter を example が直接所有（本番は `Emo2Wiring` 経由）、(b) **窓寸を自前でハンドセット**する——本番でシェル窓寸を surface 実寸へ追随させる機構は `resnap_shell_targets`→`resize_window_to`（`frame.rs:554-583`・completed `surface-resize-resnap`）だが、design 自身が「これは bin の frame 駆動であり example からは動かない（かつ C-7 で frame.rs は不触）」と述べて**迂回**する。結果、プロトコル 3. の k=1.0 assert が実証するのは「**probe 自身が `compose(1000,binds)` の extent から設定した窓寸**が OS 上でその物理 px になったこと」であり、**本番の resnap 経路の DPI 正しさではない**。

**Impact**: steering roadmap は本 spec を名指しで「本番ゴースト＋実 DPI が観測条件であり、**単発デモへの合わせ込みは無効**」と規定し、これは window-placement リジェクトの教訓そのもの（記憶 `areka-placement-real-ghost-first`）。probe は「本番 emo2 の絵を出す」点は満たすが「**本番の配線で出す**」点は満たさない。要件 7.3 が封じたかった自己整合の罠を、座標軸では封じつつ**配線軸では再現**している。

**Suggestion**: 「本 probe が実証する範囲＝emo-present の合成/表示側の等倍契約であり、本番の窓寸駆動（resnap）経路は含まない」を **Open Risk と acceptance-record の判定行に明記**して主張範囲を正直に狭める（最小対応）。より望ましくは Issue 1 のシムと束ね、probe が `Emo2Wiring` 相当の結線を通る形へ寄せる。C-7（frame.rs 不触＝W1 共有ファイル 0）との両立可否は明示的に裁定されたい。

**Traceability**: R7.3（実 DPI・本番 emo2 表示の実測証跡）
**Evidence**: design.md「Probe / Batch 契約 Input 1.–3.」「プロトコル 3.」「Coordination Notes C-7」「Open Risks 6」

### 🔴 Critical Issue 3: マウス経路の空間一致が「不要に」合流サインオフへ繰り延べられている

**Concern**: プロトコル 5. は狙点を `GetCursorPos`→`ScreenToClient` で取得し、C-4／Open Risk 3 は「**マウス由来座標との空間一致は合流サインオフのみが検証する**」と繰り延べる。しかし実測では **`PointerState.client_point: PhysicalPoint`＝「クライアント座標（物理ピクセル）」**（`crates/wintf/src/ecs/pointer/types/mod.rs:92-94`）＝**リゾルバの入力契約（窓 client 物理 px）と既に同一空間**である。すなわち本番マウス経路は設計が恐れたスケーリングを**そもそも持たない可能性が高く**、両者の一致は本 spec 内で安価に cross-check できる（同一カーソル位置で `ScreenToClient` の点と `OnPointerMoved` ハンドラが受ける `client_point` を並べてログするだけ＝**presenter 到達性を要さない**）。

**Impact**: 繰り延べの正味の効果は「本 spec の probe が本番と別経路を検証し、本番経路は1ウェーブ後まで誰も見ない」。要件 7.4 が明示的に警戒した「**双方が相手に任せて誰も検証しない**」最悪形へ、意図せず1歩近づく。逆に cross-check を入れれば C-4 の申し送りを事実で裏打ちでき、W2 の合流サインオフは残差（SHIORI 一周）に集中できる。

**Suggestion**: プロトコル 5. に **`client_point` 対 `ScreenToClient` の一致 assert（または実測表への併記）**を追加し、C-4／Open Risk 3 の文面を「マウス経路の**空間**一致は本 spec が確認済み・残るのは**イベント配送〜SHIORI 一周**」へ精緻化する。反トートロジー条件（狙点は目視由来）は不変のまま成立する。

**Traceability**: R7.3(a)（実表示窓の client 座標経路からの点）・R7.4（合流サインオフへの帰属範囲）
**Evidence**: design.md「Probe プロトコル 5.」「Coordination Notes C-4」「Open Risks 3」

## Design Strengths

1. **主張が実測に立脚しており、突合した全件が CONFIRMED**。本レビューで独立検証した範囲: `apply_show` の書込点と早期 return 構造（`presenter.rs` の `visible=true` 到達前に全失敗経路が return＝「失敗＝前値保持」が**分岐追加ゼロで成立**）／`text_slot_view().surface_size()` の実源が **`chain.size()`**（`presenter.rs:404-414`・`chain.rs:280-282`）＝k=1.0 assert の「OS 真実 vs emo 真実の**独立2源**」主張は正当／`target_map.rs` は非テストコードに `crate::` を**1件も持たない**／`emo2_boot/mod.rs:305` の `crate::is_benign_boot_error` 実呼出＝`#[path]` include 不能／前例の機構は `include!` でなく **`#[path]` モジュール宣言**（`window-placement.rs:99-113`）。とりわけ **donor が窓寸を surface0（434×687）から採りながら surface1000（382×547）を表示する**という BLOCKER 指摘（Open Risk 6）は `emo-present.rs:160-180,360-380` で実在を確認＝**設計段階で実行前に捕捉した真の欠陥**である。

2. **「正典が沈黙している」ことを一級の設計情報として扱った確定表**。C2/C7/C8/C9 を SILENT と明示し（「見落としではない」と実地確認を宣言）、各行に**設計判断＋根拠＋単一変更点＝再検証トリガ**を持たせた構造は、含端規則の是正コストを「純関数の比較式1箇所」に閉じ込めることに成功している。加えて C-5（画家則の SSP 逸脱は emo2 では**永久に検出不能**ゆえ e2e が主張できるのは「emo2 適合」であって「SSP 完全適合」ではない）を自ら明文化した知的誠実さは、記憶「あるべき姿で検討・解決/未解決を明示」の要求に正面から応えている。**なお座標契約の原点同一性は design の論証（k=1.0 のみ）より強く裏付けられる**: `compute_extent` は**原点 (0,0) 固定・負オフセットは原点クリップ**（`plan.rs:355-383`）＝合成ビットマップ原点＝サーフェス画像原点が構造的に保証され、design が「取り逃がす」とした原点ずれは実は生じない。

## Final Assessment

**Decision: GO**

**Rationale**: 設計の中核（emo-compose の純関数コア／emo-present の additive 読み口／`HitRegion` 正本／画家則と型シーム）は要件 R1–R6 を過不足なく充足し、依存方向・crate 憲章・物理 px 単一通貨・ログ規律のいずれとも整合し、実コードへの突合で反証が1件も出なかった。重大イシュー3件は**いずれも 7.3 probe の証拠力の到達範囲と本番呼び手への結線**に関するもので、純粋層の実装（＝本 spec の価値の大半かつ tasks の大半）を1行も阻害しない。Issue 1 は W2 を止めうるため design ディスカッションでの決着を推奨するが、NO-GO に相当する「基盤的な対立・不釣り合いな複雑さ」は存在しない。

**Next Steps**:
1. 設計ディスカッション（`/kiro-design-discussion areka-P0-collision-geometry`）で Issue 1–3 を裁定する。**Issue 1（呼び手側アクセスシム）を最優先**——Issue 2 の probe 配線と一体で解けるため。
2. Issue 3 は文面精緻化＋probe への1 assert 追加で閉じられる見込み（低コスト・高リターン）。
3. 裁定反映後、`/kiro-spec-tasks areka-P0-collision-geometry` へ。**research.md §11.8 の義務チェックポイント**（roadmap.md:210＝先行 spec は cue-playback マージ後の settled コードへ `/kiro-validate-design` を再実行してから tasks へ）は本レビューで充足（実測突合せ済み・crates 差分ゼロを再確認）。

### 参考: 本レビューで確認した軽微な訂正候補（イシュー化しない）

- **Open Risk 4 の悲観は過剰**: 「`hit_region.rs` の `crate::` パス禁止規律は機械的に強制されない」とあるが、`collision-probe.rs` が `#[path]` で当該ファイルを include する以上、**`cargo build --examples` が規律を機械的に強制する**（破れば example がコンパイル不能＝即発覚）。doc 明記は引き続き有用。
- **Testing Strategy の採番飛び**（9→11、#10 欠番）は research §11.6 の「テスト#10 を実装制約へ格下げ」の痕跡＝意図的。tasks 生成時に採番を詰めると読み手の混乱が減る。
