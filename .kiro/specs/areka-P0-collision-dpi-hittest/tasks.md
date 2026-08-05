# Implementation Plan

- [x] 1. 縮約算術の丸め権威を確立する
  - `ScaleRatio` に除算方向の座標縮約（DD-1 の画素中心逆写像・i128 中間・Euclid 除算・i64 飽和縮小）を新設し、丸め規約の唯一の権威とする
  - 乗算方向権威（`scaled_extent`）との対の関係・座標専用（長さの縮約には使わない）・W6.5 への申し送りを doc に明記
  - in-source 檻: k=1 全域恒等（負値・極値含む）／k=2 表（100→50・101→50）／k=5/4 表（1→1・6→5）／k=7/6 端（native 寸+1 で範囲外）／単調非減少の代表列／k<1×i64 極値の飽和
  - 完了条件: `cargo test -p areka-emo-compose` が新檻 6 種を含め緑
  - _Requirements: 2.1, 2.2, 2.5, 1.5_

- [ ] 2. ÷k＋照合の合成純関数と決定論檻
- [x] 2.1 合成純関数と縮約済み結果型を新設する
  - k を明示引数で受け、2 軸を縮約してから既存純照合関数へ完全委譲する合成純関数（重なり・反転・閉区間の意味論を再実装しない）
  - 縮約後サーフェス px 点（SHIORI 配信の正準値の出所）を結果に同梱し、crate 公開面へ再輸出
  - 既存純照合関数の Preconditions doc を「呼び手が ÷k 済み座標を渡す（or 合成関数を使う）」へ改訂・「DPI 非参照」宣言の維持を補足
  - k=1.0 恒等檻（region 完全一致・surface_point==入力）を本タスクで作成する（R3.4 の檻の正本）
  - 完了条件: k=1.0 で既存純照合関数と region が完全一致し surface_point が入力と一致する檻が緑
  - _Requirements: 1.1, 1.2, 1.3, 3.1, 3.4, 5.3_

- [x] 2.2 任意 k 注入の決定論檻を整備する
  - R3.3 の 5 分岐（領域内／別領域内／背景／矩形境界の内側 1px／外側 1px）× k=2.0・k=1.25
  - k=2.0 (100,100)≡(50,50) の固定・割り切れない縮約（k=5/4）と k=2 奇数座標の期待値固定（k=1.0 恒等檻は 2.1 が正本）
  - 重なり優先（画家則）・反転/退化矩形・負値/窓外・決定性の k≠1.0 版
  - 完了条件: GPU・実窓・sleep 非依存で全檻が緑
  - _Requirements: 3.2, 3.3, 3.5, 2.1, 2.3, 2.4, 2.5, 2.6_

- [ ] 3. presenter 配線——実適用 k の厳密消費
- [x] 3.1 production 判定入口と厳密照会を新設する
  - 私有 applied 直読（f32 非経由・derive_scale 再呼出禁止・判定ごと読取で k 更新へ自動追従）
  - applied 不在時の防御分岐（warn 1 行＋等倍続行・到達不能性の doc 明記）
  - k・縮約前後座標・解決 region の debug 1 行構造化ログ（R4.5 の観測面）
  - probe 期待ゲート用の厳密照会（applied_ratio）と ScaleRatio の再輸出
  - 既存判定メソッドの doc を「÷k の正準の呼び手は姉妹メソッド（実装済み）」へ改訂
  - 完了条件: k=1.0 で既存判定メソッドと region 一致（檻の作成は 3.2 が担う）
  - _Requirements: 1.4, 1.5, 1.6, 1.7, 4.1, 4.5, 5.3_

- [x] 3.2 配線と縮退の in-source 檻を整備する
  - 私有状態で「surface あり・applied なし」を構築し panic なし・k=1.0 同一結果・warn 経路通過を固定
  - attach のみ（未表示）target の縮退（region None・surface_point は等倍縮約値）
  - k=1.0 時に新旧判定入口の region が一致する公開面恒等檻（3.1 完了条件の検証本体）
  - 完了条件: GPU 非依存で新檻が緑
  - _Requirements: 1.6, 1.5_

- [ ] 4. bin 結線と SHIORI 配信の空間切替
- [x] 4.1 resolver を新判定入口へ切替え縮約済み点を授受する
  - 判定結果型へ surface_point を追加し全構築点を更新（presenter 不在縮退は無変換値＝等倍相当）
  - resolver の呼出切替（`#[path]` include 規律維持・依存は外部 crate のみ）
  - 冒頭 doc へ R5.4 の集約記述（受領空間・吸収点・配信空間・正典沈黙ゆえの areka 裁定・shell 限定）
  - 完了条件: 未表示 scope 縮退テスト＋surface_point 伝播テストが緑
  - _Requirements: 1.1, 1.8, 5.3, 5.4, 5.5_

- [x] 4.2 SHIORI 配信座標を surface px へ切替え throttle 空間を固定する
  - 配信座標の生成 2 箇所（move／double-click）を surface_point 値へ切替（契機・種別・頻度は不変）
  - throttle へ渡す位置は client px のまま（throttle 実装は無変更）
  - DD-IE-10 の素通し規約記述（全再掲箇所）を「resolver が ÷k を吸収・配信は surface px・throttle は client px」へ改訂
  - 檻: 配信値が surface_point であること・k=1.0 で従前値と同一・throttle 比較値が縮約されていないこと
  - 完了条件: 配線層の in-source 檻が緑
  - _Requirements: 1.8, 1.9, 6.3, 6.8, 5.3_

- [x] 5. (P) バルーン逆向き整合の明文化と二重縮約退行檻
  - コメント改訂: k=1.0 を理由とする記述の全廃→「行矩形が既に実適用 k ×済みの窓物理 px ゆえ点は無変換が正しい」＋シェル÷k／バルーン×k の逆向き等価と二重縮約禁止の併記
  - 檻 2 本: k=2.0 で持ち上げた行矩形×無変換 client 点が正しくヒット／同じ点を ÷k すると外れる（檻座標は境界から 2px 以上離し f32 誤差と無関係化）
  - 判定コードは無変更（コメント＋`#[cfg(test)]` のみ）。rebase conflict 解消時は自分の増分のみ保持し drain/status 系ハンクへ一切触れない
  - 完了条件: balloon 経路の新檻 2 本が緑・判定コードの diff ゼロ
  - _Requirements: 3.7, 5.6, 5.7, 6.4, 6.7_
  - _Boundary: バルーン明文化＋檻（input_events/balloon.rs）_

- [x] 6. (P) collision-probe の k 対応改修
  - 窓 resize 先を物理寸権威（target_physical_size）へ差替・GetClientRect 整合 assert の置換
  - read_back anchor を物理座標へ写像（中心を ×k・矩形内側 ≥2px・描画証跡であり判定証跡でない旨の明記）
  - `assert_eq!(scale, 1.0)` 撤去→env `AREKA_COLLISION_PROBE_EXPECT_K` 期待ゲート（未指定時は実測ログのみで開発機でも実行可）
  - DPI 追従駆動は追加しない（R-1 解決済み: 窓生成時に実モニタ DPI 初期化）・陳腐化コメント改訂
  - 常設 greppable ログ（k・native/physical 寸・client/surface/region）・反トートロジー維持（合成入力不使用）
  - 完了条件: `cargo build --example collision-probe` 緑＋開発機（k 任意）で期待ゲートなし実行が正常終了
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6_
  - _Depends: 3.1, 4.1_
  - _Boundary: CollisionProbe（examples/collision-probe.rs）_

- [x] 7. (P) k=1.0 限定契約の解除登記（completed 文書への日付付き追記）
  - collision-geometry design の k=1.0 限定契約と Revalidation Trigger 2 へ解除済み／消化済みの日付付き追記（既存本文は書き換えない）
  - 同 acceptance-record へ「DPI追従下の受け入れは本 spec の記録を参照」の日付付き追記
  - 完了条件: 追記が日付付きで存在し既存本文が無改変
  - _Requirements: 5.1, 5.2_
  - _Boundary: 文書改訂（specs/completed）_

- [ ] 8. 全体検証と実機サインオフ
- [x] 8.1 ワークスペース全体の非退行を検証する
  - `cargo test --workspace` exit 0（既存 GPU テスト運用の不破壊・既存檻の期待値不変）
  - 純照合関数・作者定義矩形・collision 集合経路の diff レビュー（不変確認）
  - 完了条件: workspace テストが決定論的緑
  - _Requirements: 3.6, 6.1, 6.2, 6.5, 6.6_

- [x] 8.2 実 DPI 2 水準の実機サインオフと受け入れ記録を作成する
  - OS スケール 125%（期待 k=5/4）／200%（期待 k=2）で probe を各 1 回実行（期待ゲート env・有界 auto-exit・絶対パス起動）
  - 頭・胸・背景を人間の目視のみで狙い、client/surface/region ログと目視の一致を突合・2 実行の物理寸が互いに異なる証跡を採取
  - 本番 emo2 実ゴーストで shell target の実 k を debug ログ grep（R-2 の消化）
  - acceptance-record.md 新設（判定・実測値・実施条件・不一致欄。不一致時は是正して再実施するまで完了としない）
  - 完了条件: acceptance-record.md に 2 水準の合格記録が存在する
  - _Requirements: 4.1, 4.2, 4.6, 4.7, 4.8_

## Implementation Notes

- 2.1: `hit.rs` の恒等檻 doc（`scaled_identity_matches_hit_region_exactly` 直上）に「×k の誤挿入も落とす」との過剰主張がある。k=1.0 では ×k も恒等ゆえ検出不能。×k 誤挿入の検出は 2.2 の任意 k 檻が担うので、2.2 で文言を是正すること。
- 8.2: **2 水準サインオフ合格**（2026-08-05）。125%＝k=5/4・physical `478x684` ／ 200%＝k=2/1・physical `764x1094`（native は両者 `382x547` で不変）＝**互いに異なる拡大寸**の証跡が初めて成立（`collision-geometry` Task 4.2 却下の観測条件を突破）。DD-1 の丸め規約が実機で厳密一致（k=5/4 で 9/9・k=2 で 5/5）。閉区間の内外も 1px 単位で保存（k=2: Head 下端 surface y=130 当たり／131 外れ）。脚 B は本番 emo2 を絶対パス起動して SHIORI 実結線成立（`0x8007007E` 0 件）・shell `TargetId(0)` の実 k が `ScaleRatio { num: 2, den: 1 }`＝**R-2 消化**・`hit_region_client` の debug を 2271 行採取（Head 638／Bust 445／None 224）＝**本番バイナリが新 ÷k 入口を通っている証跡**・開発者が撫で／さわり反応を直接目視確認。
- 8.2: **probe には手動終了の手段が無い**（実機で発覚・doc を是正済み）。かつて終了を担った stand-in `spawn_ghost_windows` の `OnPointerPressed(on_ghost_pressed)` は `areka-P0-input-events` task 2.7 で退役し、正典の脱出口（Ctrl+左ダブルクリック・DD-IE-7）は `input_events::attach_char_pointer_handlers` へ移ったが、同関数は `pub(crate)` かつ内部が `crate::` パスを使うため example から `#[path]` include できない。**probe 実行時は必ず `AREKA_APP_SMOKE_EXIT_MS`（推奨 180000）を与えること。** 脱出口の結線は本 spec の射程外（`input_events` の `#[path]` include 可能化が要る）。
- 8.2: **probe は可視の端末で走らせること。** 出力をログファイルへリダイレクトすると、`region=` の変化を見ながら狙う本来の目視突合ができなくなる。今回は「狙う順序を先に宣言 → `region=` の時系列区間の並びと照合」という形で代替した（結果を見る前に期待値を宣言するので自己整合の罠は生じない）。本番 emo2（脚 B）は撫で反応という可視フィードバックがあるため、その場で直接判定できる。
- 8.1: `cargo test --workspace` は本増分適用後 **6 実行中 5 回 exit 0**（85 binaries・4756 passed・0 failed・33 ignored で件数完全一致・GPU テスト 207 本実走）。**残る 1 回の赤は本増分と因果独立の既存 flake**——`areka-seriko` の `capture_logs`（`crates/areka-seriko/src/actor.rs:1948`）が `rebuild_interest_cache()` 未硬化で、tracing callsite interest cache のプロセス共有・first-thread-wins により確率的に捕捉 0 件となる（[[areka-log-cage-harness-blindspots]]）。因果独立の証跡: `git diff --stat 8112295 ece56eb -- crates/areka-seriko/` が空（当該テストとハーネスは base とバイト同一）／`areka-emo-compose` の増分 717 行に tracing 呼出 0 行／当該テストの最終更新は分岐前の `70bd1b3`。R3.6 の主語は「本仕様のテスト増分」で義務は「適用後**も**緑に**保つ**」＝保存義務ゆえ**充足**と裁定。
- 8.1→**別 spec への申し送り（未登記・要対応）**: 上記 flake の所有 spec は `areka-P0-test-cage-determinism`（W6.9）で、機序は `brief.md:19-21` に正確に記述済み・射程も「全 crate 横断」（`brief.md:70-71`・`:91`）。**しかし未硬化サイト表（`brief.md:25-32` ＋追記(58)）は `crates/areka/src/**` の 7 件のみで `crates/areka-seriko/src/actor.rs` が未登記**。同 brief 自身「着手時に実測して取りこぼし確認」と書いており表はドリフトする。**当該 brief の①インベントリへ `crates/areka-seriko/src/actor.rs`（`capture_logs` は `:1948`・`:1946` に「スレッドローカル `with_default` ゆえ並行テスト安全」という①の目印そのものの偽の否定コメントあり・tracing 呼出は実測 31〜32 件）を追記すること。** 本 spec の boundary 外ゆえここでは是正していない。
- 6→**8.2 への申し送り（必読）**: **開発機の実表示スケールは 200%（実適用 k=2/1）と実測済み**。probe 実行の実測値は `k=2/1 native=382x547 physical=764x1094`。8.2 の 2 水準は「現状の 200% で 1 回（`AREKA_COLLISION_PROBE_EXPECT_K=2`）」＋「OS 表示スケールを 125% へ変更して再実行（`=5/4`）」で満たす。期待ゲートは不一致時 exit 101 の hard assert で loud fail することを実測済み。
- 6→8.2 への申し送り: R4.6 は**二脚**に分かれる。probe が担うのは**ヒットテスト目視の脚**（probe は `pasta.dll` を一切ロードしないので絶対パス起動の失敗経路自体が存在しない）。**本番 emo2 実ゴーストを絶対パスで起動する脚は 8.2 の担当**。acceptance-record では**両脚を別項目として記録**し、probe 実行を絶対パス起動の証跡に流用しないこと。
- 6: probe は `attach_target` へ author_dpi=96 を直書きしている。emo2 fixture は shell descript に `seriko.dpi` 宣言が無く既定 96 ゆえ**たまたま一致**するが、`seriko.dpi` を宣言するゴーストへ差し替えると採寸 k₀ と表示 k が食い違う。そのときは `PreparedPlacement::author_dpi.shell`（搬送口は実在）へ差し替えること。本 spec の射程外ゆえ実装変更はしていない。
- 6: 本 spec は doc の事実誤認で **4 回** 差し戻しになった（2.2 で 2 件・6 で 2 件）。**doc に「どのコードが何を読む／どの誤実装をどの点が殺す」と書くときは、書く前に該当コードを開いて file:line で裏取りすること。** 未確認の推測を書かない。
- 4.1→**4.2 への申し送り（必読）**: 4.1 で `RegionSource::Mock` の 12 クロージャを `surface_point: (x, y)`（k=1.0 恒等の忠実な模型）へ更新した。このため既存ハンドラ檻では `surface_point` が生の client 点と**数値的に区別できない**。4.2 で `MouseInput{x,y}` を `surface_point` へ切替えても、これらの檻は切替の前後で同じ色のまま＝何も証明しない。**4.2 は `surface_point` が `(x, y)` と意図的に異なる mock（例 `(x / 2, y / 2)`）を最低 1 本導入し、「配信値が surface_point であること」と「throttle へ渡る pos が client px のままであること」の両方を反証可能にすること**（design の bin テスト項目 2）。
- 4.1→4.2 への申し送り: `input_events/mod.rs` の DD-IE-10 記述（:98 / :109 / :144 / :184 / :296 付近の「素通し＝DPI 変換なし」「k=1.0 契約」）は 4.1 では意図的に未改訂。現在は実態と食い違っているので 4.2 で必ず消化すること（`DD-IE-10` を grep して特定）。
- 3.2: ログ檻で「出ないこと」を主張するときは、**陽性（発火）と陰性（無音）を同一 `with_default` スコープ・同一 callsite 内で観測**すること。別スコープで陰性だけ見ると tracing の callsite interest キャッシュで恒真になる（[[areka-log-cage-harness-blindspots]]）。3.2 ではこの形で書き、述語潰し変異のとき陰性側の warn が実際に捕捉列へ現れることまで実測している。
- 3.1: R1.6 防御分岐の述語は **「`applied == None` かつ表示 surface が存在する」**（DD-5 が正典・design の Error Handling 表が未登録/未表示を「正常縮退・ログ不要」と分類しているため）。`applied.is_none()` 単独にすると未表示 scope 上のマウス移動ごとに warn が出るログ洪水になる。3.2 の R1.6 檻はこの述語で構築すること（未登録・未表示では warn 0 件であることも併せて固定する）。
- 3.1: `lib.rs` の `pub use areka_emo_compose::ScaleRatio;` がローカル module の `pub use` 群より後ろにあり rustfmt のグループ順序に反する（crate は元々広く fmt 非準拠・fmt ゲートは存在しない）。将来 fmt をかけるときに `pub use balloon::…` の上へ移すこと。
- 2.2: 檻の doc に「どの誤実装をどの点が殺すか」を書くときは、必ず実測（mutant を作って落ちる檻を記録）してから書くこと。round 1 で 2 件の事実誤認（うち 1 件は隣接 assert と自己矛盾）が出た。丸め檻は k=2 系と k=5/4 系で殺せる mutant が異なる（k=2 では DD-1 と素の floor が一致するため、素の floor を殺せるのは k=5/4 系のみ）。
- 2.1: `scale::tests::mul_degradation_emits_warn_log`（`scale.rs`・本 spec の増分ではない既存檻）が全体並列実行で稀に失敗した報告あり。疑いは tracing callsite interest キャッシュ競合（[[areka-log-cage-harness-blindspots]] クラス）。レビュー時 4/4 は再現せず。8.1 の `cargo test --workspace` 決定論性（R3.6）で再評価すること。
