# 設計の検証: shell-scale-linkage

> 2026-11-01・本ブランチ `claude/kiro-start-shell-scale-linkage-773a37`（`8095e97e`）の design.md を、requirements.md（末尾の開発者の裁定を含む）・research.md・steering（`product.md`・`tech.md`・`structure.md`・`logging.md`・`roadmap.md` の C15 の行）と実物のコードに照らして読んだ。対話なしで出した判定。

## 設計の検証のまとめ

設計は、倍率の変わりを「絵に倍率をかけたその 1 か所」で積み、フレームの最後の相が表とイベントへ出す形で、毎フレームの見比べも 1 フレームの遅れも使っていない。主な主張はコードで確かめて合っていた。実装へ進めてよいが、下の 3 点を設計の討議で決めておく必要がある。

## コードで確かめたこと

| 設計の主張 | 確かめた結果 |
|---|---|
| `applied` を新しい値で書くのは `apply_show` だけ | 合っている。`presenter/show.rs` の `target.applied = Some(scale)` が唯一。ほかの書き手は `presenter/replace.rs` の引き継ぎ（`carried.applied`）だけで、設計はそれを起点の欄から外している |
| 差し替えは `attach_target` を通るので起点が空になる | 合っている。`apply_replace` は `detach_target` → `attach_target`（`PresentTarget` を新しく作る）→ 名指しの欄だけを書き戻す。新しい欄は書き戻さなければ空のまま |
| 倍率を書く相はどれも新しい相より前 | 合っている。`emo2_frame_system` の中で attach・dpi（`refresh_scale`）・drain・差し替え・バルーンの可視性（`show_target`）が並び、最後が `run_status_report_phase`。`Emo2Wiring` を取り出すのはこの系だけで、ほかに `presenter.apply` を呼ぶ本番の道は無い＝同じフレームで表とイベントが出る |
| 起動のすぐ後の DPI の補正は送らない | 同じフレームに起きる補正（`attach` → `dpi` の相）は、計画が「最初の `before`（空）・最後の `after`」に畳むので送らない。正しい |
| `rewire_runtime_consumers` は `Emo2Wiring` を読める | 合っている。呼び手は `ghost_session.rs` の起動（`wire_emo2_boot` が入れた後）と `rebuild_runtime_in_place`（`ghost_quit_system`・`drain_change_requests` から）で、どちらもフレームの系の外 |
| 受け口の置き場に材料が揃う | 合っている。`areka-ghost/src/runtime.rs` の `PropSetCueSink` を足す行の所に `sylphya_reader`・`ghost_asker`・`kanade_tx` が在る |
| `syncscale` の読み直しの印 | `DefaultPlacementBasis` を入れるのは `placement/mod.rs`（起動）と `frame/switch.rs`（シェル・descript の読み直し）の 2 か所。ゴーストの切り替え・読み直しは起動を通る。合っている |
| 行数 | `placement/mod.rs` 970・`ghost_session.rs` 940・`schedule/events.rs` 913・`actor.rs` 846・`runtime.rs` 822。足す量はどれも 1000 行の内 |
| 数の檻・型 | 許可の表は 64（`events_change_tests.rs`）・受け手の表は 42・`DPI.dpi_x` は `u16`・`ScaleRatio` は既約の分子と分母を持つ・`validate_notify` は空→`EmptyEvent`・CR/LF/NUL→`ControlCharacter`。設計の書き方と合う |
| ウェーブの兄弟と同じファイルを触らないか | 兄弟のブランチの差分で確かめた。⑤ は emo-atlas の `lib.rs`・emo-compose の `fold.rs`・`world.rs`・`base_image.rs`、③ は `alert.rs`、⑥ は kanade の `schedule/resources.rs` だけ。本設計のファイルとは重ならない |
| 外のクレート | 新しいもの・`[patch]`・フォークは無い |

## 重大な点（3 つ）

🔴 **重大な点 1**: ウェーブ C15 の表の ① に載っていないファイルを触る
**中身**: 本設計は `areka-ghost`（`prop_get_sink.rs`・`runtime.rs`・`lib.rs`）・`areka-emo-present` の `presenter/` 以下・`presenter.rs`・`lib.rs`・`areka-emo-compose/src/scale.rs`・`frame/scale_notice.rs`・`target_map.rs`・`ghost_session_scope_rect.rs` を触る。どれも `roadmap.md` の C15 の ① の欄に無い。代わりに欄にある `emo2_boot/{mod,runtime_boot}.rs`・`frame/dpi.rs`・`placement/config.rs` は触らない。
**影響**: ウェーブの約束は「触るファイルの欄」で並走の安全を保っている。兄弟との重なりは今は 0 と確かめたが、開発者の承認と欄の書き直しが無いまま実装へ進むと、次の棚卸の測りが古い欄を基に組まれる。
**提案**: 設計の討議で、この入れ替え（`areka-ghost` に受け口・表示層に記録の口）を開発者に承認してもらい、`roadmap.md` の C15 の ① の欄を設計の File Structure Plan どおりに直す（直すのは完了の手続きか次の棚卸のどちらかを決める）。
**つながる要件**: Boundary Context の Adjacent expectations（ウェーブ C15 の約束）
**設計の箇所**: File Structure Plan の「ウェーブの表 ① に無いファイル（開発者の承認を仰ぐ）」

🔴 **重大な点 2**: 装着の後・定常に入る前に倍率が変わったときの扱いが決まっていない
**中身**: 「初めて決まった」の見分けは装着ごとの起点だけで、装着の後の最初の表示のフレームしか覆わない。起動・ゴーストの切り替えの途中で、最初の表示より後のフレームに窓が別の拡大率の画面へ移る・DPI の補正が遅れて届くと、起点があるので `OnShellScaling` を送ろうとする。kanade はまだ定常でない（`Phase::BootMain` など）ので `warn!(raise_event_not_steady)` で捨てる。
**影響**: 正常な起動の道に警告の行が出る（steering `logging.md` の「警告は異常」とずれる）。要件 5.7 の「起動・切り替えではイベントを送らない」を、どの範囲まで「起動」と見るかが設計に無い。同じフレームの檻は「最初のフレームは知らせ 0 件」しか固定していない。
**提案**: ⑴ 定常の前の倍率の変わりは「起動の一部」として、送らずに表だけ書く（UI の側で GhostSlot の運行の段を見るか、kanade が捨てるときの段を `debug!` にする）か、⑵ 開発者の裁定「描画スケールが変わったときに出る」どおり送ろうとし、捨てる記録を `debug!` に下げるか、を討議で決める。どちらでも「最初の表示の後のフレームで DPI が変わり、まだ定常でない」場面の決定論のテストを 1 本足す。1 フレーム遅らせる解は取らない。
**つながる要件**: 5.7・5.1・7.3
**設計の箇所**: Flow 3（装着ごとの起点）・ScaleNoticePhase の Idempotency & recovery・Testing Strategy の Integration Tests 2

🔴 **重大な点 3**: 見えていないバルーンの倍率と、出し直したときの `OnBalloonScaling`
**中身**: `refresh_scale` は見えていない窓を描き直さない（`presenter/refresh.rs` の `!visible` の早期の戻り）。バルーンが隠れている間にシェルが別の拡大率の画面へ移ると、表のバルーンの行は前の画面の倍率と DPI のまま残り、次の台詞でバルーンを出す（`show_target` が今の DPI で倍率を出し直す）フレームで初めて `OnBalloonScaling` が出る。ゴーストがその応答で台本を返すと、areka の今日の規則（`value_replaces_active_talk`・要件 1.7）で、いま始まった台詞を置き換える。
**影響**: 値と絵は食い違わない（要件 4.2 は守れる）が、設計の流れと檻にこの場面が無い。SSP 向けのほかのゴーストで、画面をまたいだ後の最初の台詞が応答で切られることがある（ぱすたは応答を空にしたので困らない）。
**提案**: 「隠れている間は前の倍率のまま・出し直したフレームで送る」を意図した形として設計の Flow 2・3 に書き、決定論のテスト（隠れたバルーン → DPI の変わり → 出し直しのフレームで 1 件）を表示層の檻と `scale_notice` の檻に足す。台詞を切られるのを避けたいかどうかは開発者に確かめる（避けるなら、送る順や応答の扱いの決めが要る）。
**つながる要件**: 3.2・4.2・5.2・5.5・1.7
**設計の箇所**: PresenterScaleRecord の Responsibilities・Flow 2・Flow 3・Testing Strategy

## 設計の強み

1. **プッシュの形が正しい場所に置かれている。** 倍率の変わりを、`applied` を書くただ 1 か所で「装着ごとの起点」と比べて積み、フレームの最後の相が取り出す。毎フレームの見比べをせず、切り替えの見分けも別の印を配らず状態の形（起点の欄）で解き、同じフレームで表を書いてからイベントを送るので、応答の台本の照会にも新しい値が返る。コードの相の並びと照らして成り立つ。
2. **既存の先例に乗り、足す量が小さい。** 受け口は `PropSetCueSink`、値の口は `register_pull`、イベントは `RaiseEvent`、名前の検査は `validate_notify`、記録は `pending_resize` の形を借りる。行の上限に近いファイルへの足しは 1〜3 行に収まり、外のクレートは足さない。失敗の分岐はすべて記録の語と段が決まっている。

## 判定

**判定: GO**

**理由**: 開発者の原則（変わる時点が分かるならプッシュ・1 フレーム遅らせない・外のクレートはそのまま・1000 行・記録を必ず残す・判断の分岐は決定論のテスト）との食い違いは見つからず、主な主張はコードで確かめられた。3 点は設計の討議で決めて書き足せば済み、作りの土台は変えない。

**次の手順**:
- 設計の討議（`/kiro-design-discussion shell-scale-linkage`）で上の 3 点を決め、design.md に書き足す（点 1 は開発者の承認と `roadmap.md` の欄の直し方・点 2 は定常の前の扱いと記録の段・点 3 は意図した形として書くかどうか）。
- 決めた後、`/kiro-spec-tasks shell-scale-linkage` でタスクを作る。
