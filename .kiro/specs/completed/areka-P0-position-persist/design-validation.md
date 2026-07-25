# 設計バリデーションレポート: areka-P0-position-persist

> 実施: 2026-07-24（kiro-validate-design・非対話モード）
> 対象: design.md（2026-07-24 生成）／requirements.md（2026-07-24 改訂・確定）／research.md（§4 開発者ディスカッション決定 #1・#2 は既決として尊重）
> 検証方法: design-review.md の 4 基準（既存アーキテクチャ整合・一貫性・拡張性/保守性・型/インターフェース）＋**実コードベースへのスポットチェック**（origin/main b0de116 ベースの現行ワークツリー）

## レビューサマリ

本設計は「新機構を作らず実物を結線する」層に徹しており、引用された統合点をすべて実コードで照合した結果、**行番号レベルで一致**する例外的に高いグラウンディング精度を確認した（設計主張と実コードの矛盾は検出されなかった）。W4 並走契約（`measure.rs`／`input_events/`／`emo2_boot/` 不触）は A1 採用と boot() 内 sink 登録により構造的に守られ、既決事項（全アンカー保存・トーク末尾 SET キュー）も設計へ忠実に反映されている。残る懸念は実装局面の精度に関わる 2 点のみで、いずれも NO-GO 級ではない。

### コード照合結果（抜粋・全て一致）

| 設計主張 | 実測 |
|---|---|
| `on_char_drag_end` follow.rs:319／`on_balloon_drag` :443／`project_anchor` :143／`move_window_to` :502／`work_area_for_window` :852 | 一致 |
| spawn.rs `if !p.anchor.is_free()` ガード :230-233（Free 窓 OnDragEnd 未結線）・`CharWindowMarker` :72・`BalloonWindowMarker` :80 | 一致（既存檻 :719/:740 の「付けない」assert は反転更新が必要＝File Structure Plan の機械的追随欄に既記載） |
| kanade `on_prefetch_reply` boot.rs:131（無条件 OnFirstBoot 発行 :180）・`to_baseware_version` :197（config 既受領・全経路単一点）・204 フォールスルー :74-79 | 一致 |
| `events::on_first_boot` events.rs:91（Ref0="0" 固定）・檻 :311 | 一致 |
| sylphya `persist_put` actor.rs:481・`barrier` :491・`close` :508・`load_scope`/`save_scope` persist/mod.rs:183/:231・`FakePersistIo`・正準 key 檻 | 一致 |
| ghost `boot()` runtime.rs:386・publisher private :154・`spawn_kanade` :461・`spawn_dispatcher` :474・shutdown 内 sylphya close :322・`profile_areka_root`（pub）sylphya_wiring.rs:85・sink 先例 :142 | 一致 |
| `StartTalk { talk_id, script }`（epilogue は additive 新設）・`BootCueSink`＝`CueSink + Clone + Send + 'static` blanket（sink.rs:35-43） | 一致 |
| sakura drive.rs `on_start` :173（parse :183→compile :186→空判定 :190）＝「compile 後・空判定前」の挿入位置は構造成立 | 一致 |
| dola `command_carrier`/`as_command_carrier`（command.rs:201/:213）・`CueSheet::new` の `sort_by`（sheet.rs:41-42＝安定ソート） | 一致 |
| `consumer_ledger.rs` は emo2_boot/ 配下（W4 不触面）＝登記後送の判断は正当 | 一致 |

## Critical Issues（最大 3）

🔴 **Critical Issue 1**: バルーン DragEnd の保存値源（offset の鮮度）
**Concern**: `on_balloon_drag_end` は in-session `BalloonFollow.offset`（最後の `on_balloon_drag` で更新された値）を読んで保存する設計だが、char 側には「最終確定位置は最後の OnDrag 配信とずれ得る」前提で最終カーソル位置に再適用する既存檻が存在する（follow.rs:1875 `on_char_drag_end_applies_policy_at_final_cursor`）。バルーンで同種のずれがあると、1 ドラッグの最終微小移動が保存から漏れ得る。
**Impact**: 保存位置が「ユーザーが最後に置いた位置」から数 px ずれる可能性（2.1 の即時確定意味論の精度低下）。
**Suggestion**: `on_balloon_drag_end` では DragEnd イベントの最終位置（または現在の `WindowPos`）から `balloon_pos − char_pos` を再導出して保存する（`on_balloon_drag` と同じ導出式）。tasks 生成時にこの導出源を明記し、檻（DragEnd 最終位置→保存値等価）で固定すること。
**Traceability**: 2.1, 8.1
**Evidence**: design.md「C2/C3: DragEnd 観測点の保存結線」（「現在 `offset`（左上基準…）」の記述）

🔴 **Critical Issue 2**: `barrier()` のフェンス範囲と E2-lite 檻の観測経路
**Concern**: research §1.2 は `barrier()` を「**同一送信端**の投函全反映を待つフェンス」と記録する。shutdown の E2-lite は GhostRuntime 保持の publisher から呼ぶが、実際の保存投函は `PersistWiring` の **clone**（UI スレッド）から行われる。mpsc 単一 FIFO キューゆえ「enqueue 済みメッセージは Barrier より先に処理される」は送信端をまたいで成立し、かつ shutdown 時点で UI 送信は静止済み——だが design はこの越境成立の根拠を明文化しておらず、統合檻 3（終了時フラッシュ）が clone 経由 put を含むかも未指定。
**Impact**: 檻が runtime 側 publisher の put だけで書かれると、R1.2 の安全網が実経路（UI clone→shutdown barrier）を検証しないまま緑になる恐れ。
**Suggestion**: 統合檻 3 を「`PersistWiring` の clone から n×DragEnd 相当 put→`barrier()`/`close()`→ファイル最終値一致」で書くことを tasks に明記し、design の軸E 節へ「clone 送信端の put も単一 FIFO で barrier に先行処理される（shutdown 時 UI 静止済み）」の 1 行根拠を追補する。
**Traceability**: 1.2, 8.1
**Evidence**: design.md「軸E」「C5 shutdown() 増分」「Testing Strategy / Integration 3」

（第 3 の Critical Issue なし——spawn.rs 既存檻の assert 反転・`on_prefetch_reply` への config 引数追加等はすべて設計に既記載の機械的追随であり、Critical に該当しない）

## Design Strengths

1. **実測グラウンディングの精度と反証可能性**: 引用された統合点（follow/spawn/boot/events/actor/persist/drive/command/sheet）が全数スポットチェックで実コードと一致。A1 採用（placement シーム先読み）により起動順序・W4 並走契約・`prepare_never_reads_or_writes_ghost_dat` 檻の精神を**不変のまま**要件を満たす経路が、行番号つきで検証可能に書かれている。
2. **書込経路の構造的単一化**: 位置の永続ライターを DragEnd 2 点に限定し、SET sink 側は key 統制（カウンタ族限定・WindowPos/BalloonOffset 拒否）で 1.9 の単一ライター規律を cue 側から防衛。「完走時のみ記録」（3.4）を CueSheet 末尾 cue の既存破棄意味論（`CuePlayer::stop()`）に載せ、新規監視機構ゼロで要件意図を構造で実現している。steering 規律（汎用キャリア 1 本・kanade→sylphya 依存禁止・log-first）への適合も明示的。

## Final Assessment

**Decision: GO**

**Rationale**: 実コード照合で設計主張の矛盾ゼロ・アーキテクチャ整合（依存方向・W4 並走契約・steering 規律）に致命的欠陥なし。指摘 2 件はいずれも tasks 生成時に反映可能な実装精度・檻設計の補強であり、設計の骨格を変えない。

**Next Steps**:
1. 設計ディスカッション（kiro-design-discussion）で Issue 1・2 の扱いを確認（design.md の当該 2 箇所の追補 or tasks への注記で足りる）
2. `/kiro-spec-tasks areka-P0-position-persist` で実装タスク生成（Issue 1 の保存値源・Issue 2 の檻経路を task 記述に織り込む）
