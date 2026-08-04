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

- [ ] 3.2 配線と縮退の in-source 檻を整備する
  - 私有状態で「surface あり・applied なし」を構築し panic なし・k=1.0 同一結果・warn 経路通過を固定
  - attach のみ（未表示）target の縮退（region None・surface_point は等倍縮約値）
  - k=1.0 時に新旧判定入口の region が一致する公開面恒等檻（3.1 完了条件の検証本体）
  - 完了条件: GPU 非依存で新檻が緑
  - _Requirements: 1.6, 1.5_

- [ ] 4. bin 結線と SHIORI 配信の空間切替
- [ ] 4.1 resolver を新判定入口へ切替え縮約済み点を授受する
  - 判定結果型へ surface_point を追加し全構築点を更新（presenter 不在縮退は無変換値＝等倍相当）
  - resolver の呼出切替（`#[path]` include 規律維持・依存は外部 crate のみ）
  - 冒頭 doc へ R5.4 の集約記述（受領空間・吸収点・配信空間・正典沈黙ゆえの areka 裁定・shell 限定）
  - 完了条件: 未表示 scope 縮退テスト＋surface_point 伝播テストが緑
  - _Requirements: 1.1, 1.8, 5.3, 5.4, 5.5_

- [ ] 4.2 SHIORI 配信座標を surface px へ切替え throttle 空間を固定する
  - 配信座標の生成 2 箇所（move／double-click）を surface_point 値へ切替（契機・種別・頻度は不変）
  - throttle へ渡す位置は client px のまま（throttle 実装は無変更）
  - DD-IE-10 の素通し規約記述（全再掲箇所）を「resolver が ÷k を吸収・配信は surface px・throttle は client px」へ改訂
  - 檻: 配信値が surface_point であること・k=1.0 で従前値と同一・throttle 比較値が縮約されていないこと
  - 完了条件: 配線層の in-source 檻が緑
  - _Requirements: 1.8, 1.9, 6.3, 6.8, 5.3_

- [ ] 5. (P) バルーン逆向き整合の明文化と二重縮約退行檻
  - コメント改訂: k=1.0 を理由とする記述の全廃→「行矩形が既に実適用 k ×済みの窓物理 px ゆえ点は無変換が正しい」＋シェル÷k／バルーン×k の逆向き等価と二重縮約禁止の併記
  - 檻 2 本: k=2.0 で持ち上げた行矩形×無変換 client 点が正しくヒット／同じ点を ÷k すると外れる（檻座標は境界から 2px 以上離し f32 誤差と無関係化）
  - 判定コードは無変更（コメント＋`#[cfg(test)]` のみ）。rebase conflict 解消時は自分の増分のみ保持し drain/status 系ハンクへ一切触れない
  - 完了条件: balloon 経路の新檻 2 本が緑・判定コードの diff ゼロ
  - _Requirements: 3.7, 5.6, 5.7, 6.4, 6.7_
  - _Boundary: バルーン明文化＋檻（input_events/balloon.rs）_

- [ ] 6. (P) collision-probe の k 対応改修
  - 窓 resize 先を物理寸権威（target_physical_size）へ差替・GetClientRect 整合 assert の置換
  - read_back anchor を物理座標へ写像（中心を ×k・矩形内側 ≥2px・描画証跡であり判定証跡でない旨の明記）
  - `assert_eq!(scale, 1.0)` 撤去→env `AREKA_COLLISION_PROBE_EXPECT_K` 期待ゲート（未指定時は実測ログのみで開発機でも実行可）
  - DPI 追従駆動は追加しない（R-1 解決済み: 窓生成時に実モニタ DPI 初期化）・陳腐化コメント改訂
  - 常設 greppable ログ（k・native/physical 寸・client/surface/region）・反トートロジー維持（合成入力不使用）
  - 完了条件: `cargo build --example collision-probe` 緑＋開発機（k 任意）で期待ゲートなし実行が正常終了
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6_
  - _Depends: 3.1, 4.1_
  - _Boundary: CollisionProbe（examples/collision-probe.rs）_

- [ ] 7. (P) k=1.0 限定契約の解除登記（completed 文書への日付付き追記）
  - collision-geometry design の k=1.0 限定契約と Revalidation Trigger 2 へ解除済み／消化済みの日付付き追記（既存本文は書き換えない）
  - 同 acceptance-record へ「DPI追従下の受け入れは本 spec の記録を参照」の日付付き追記
  - 完了条件: 追記が日付付きで存在し既存本文が無改変
  - _Requirements: 5.1, 5.2_
  - _Boundary: 文書改訂（specs/completed）_

- [ ] 8. 全体検証と実機サインオフ
- [ ] 8.1 ワークスペース全体の非退行を検証する
  - `cargo test --workspace` exit 0（既存 GPU テスト運用の不破壊・既存檻の期待値不変）
  - 純照合関数・作者定義矩形・collision 集合経路の diff レビュー（不変確認）
  - 完了条件: workspace テストが決定論的緑
  - _Requirements: 3.6, 6.1, 6.2, 6.5, 6.6_

- [ ] 8.2 実 DPI 2 水準の実機サインオフと受け入れ記録を作成する
  - OS スケール 125%（期待 k=5/4）／200%（期待 k=2）で probe を各 1 回実行（期待ゲート env・有界 auto-exit・絶対パス起動）
  - 頭・胸・背景を人間の目視のみで狙い、client/surface/region ログと目視の一致を突合・2 実行の物理寸が互いに異なる証跡を採取
  - 本番 emo2 実ゴーストで shell target の実 k を debug ログ grep（R-2 の消化）
  - acceptance-record.md 新設（判定・実測値・実施条件・不一致欄。不一致時は是正して再実施するまで完了としない）
  - 完了条件: acceptance-record.md に 2 水準の合格記録が存在する
  - _Requirements: 4.1, 4.2, 4.6, 4.7, 4.8_

## Implementation Notes

- 2.1: `hit.rs` の恒等檻 doc（`scaled_identity_matches_hit_region_exactly` 直上）に「×k の誤挿入も落とす」との過剰主張がある。k=1.0 では ×k も恒等ゆえ検出不能。×k 誤挿入の検出は 2.2 の任意 k 檻が担うので、2.2 で文言を是正すること。
- 3.1: R1.6 防御分岐の述語は **「`applied == None` かつ表示 surface が存在する」**（DD-5 が正典・design の Error Handling 表が未登録/未表示を「正常縮退・ログ不要」と分類しているため）。`applied.is_none()` 単独にすると未表示 scope 上のマウス移動ごとに warn が出るログ洪水になる。3.2 の R1.6 檻はこの述語で構築すること（未登録・未表示では warn 0 件であることも併せて固定する）。
- 3.1: `lib.rs` の `pub use areka_emo_compose::ScaleRatio;` がローカル module の `pub use` 群より後ろにあり rustfmt のグループ順序に反する（crate は元々広く fmt 非準拠・fmt ゲートは存在しない）。将来 fmt をかけるときに `pub use balloon::…` の上へ移すこと。
- 2.2: 檻の doc に「どの誤実装をどの点が殺すか」を書くときは、必ず実測（mutant を作って落ちる檻を記録）してから書くこと。round 1 で 2 件の事実誤認（うち 1 件は隣接 assert と自己矛盾）が出た。丸め檻は k=2 系と k=5/4 系で殺せる mutant が異なる（k=2 では DD-1 と素の floor が一致するため、素の floor を殺せるのは k=5/4 系のみ）。
- 2.1: `scale::tests::mul_degradation_emits_warn_log`（`scale.rs`・本 spec の増分ではない既存檻）が全体並列実行で稀に失敗した報告あり。疑いは tracing callsite interest キャッシュ競合（[[areka-log-cage-harness-blindspots]] クラス）。レビュー時 4/4 は再現せず。8.1 の `cargo test --workspace` 決定論性（R3.6）で再評価すること。
