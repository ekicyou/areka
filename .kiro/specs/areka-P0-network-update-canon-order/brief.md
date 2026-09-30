# Brief: areka-P0-network-update-canon-order

> 2026-09-30 起票（`/kiro-discovery`・`areka-P0-network-update` の完了時の棚卸）。段は **α 後**（開発者指示「ネットワーク更新イベント関係で最新資料が入手できた。本仕様に関しては α 後の対応とし、実装完了時の申し送りのみ」）。

## Problem

ゴーストの作者は、ネットワーク更新の間にゴーストへ届くイベントの順と間合いを前提に台詞を書く（「ダウンロードします」「照合します」「更新したで！」）。完了した `areka-P0-network-update` は、当時の正典（各イベントの定義だけ）と開発者の確定（2026-09-30）で流れを決めた。その後、正典 ukadoc に**イベントの発生順序**が明文化された（コミット https://github.com/ukatech/ukadoc/commit/1ea881a40f9a99821c92b69ded742986de0755b0 ・`manual/list_shiori_event.html#caption_updateorder`）。areka の流れはこれと 5 点で食い違い、SSP 向けに書かれた辞書では台詞が途中で切られる・取り直しが効かない・総括が期待と違う形で届く。

## Current State

`areka-P0-network-update`（完了）の実装（正本はその spec の tasks.md の Implementation Notes の末尾と `doc/COMPAT_ARCHITECTURE.md` §8）:

- 手続き `crates/areka/src/update/procedure.rs`（`run_order`・`run_target`）がエンジン `areka_update::run` を 1 周回し、進捗をイベントへ写して送る。台詞の再生は待たない（kanade が単一 slot で置き換える）
- ゴーストの成功の `OnUpdateComplete` と総括は「後送りの列」に入り、何か変わっていれば読み直し（同じゴーストへの知らせなしの切替）の後、新しいゴーストの起動の知らせ（kanade `BootOrigin::Updated`）と切替の終わりに送る。シェル・バルーンの `OnUpdateOther*` は読み直しの前に古いゴーストへ送る
- 総括は入口が `Current` なら `OnUpdateResult`、`Other` なら `OnUpdateResultEx` の片方だけ
- エンジンは MD5 が合わなければその場で失敗（取り直さない）
- 自動更新（理由 `auto`）は無い（入口は `manual`・`script` だけ）

正典の発生順序との差（5 つ）:

1. MD5 が合わなければ同じファイルを `OnUpdate.OnDownloadBegin` から取り直し、上限を超えたら `OnUpdateFailure(md5 miss)`
2. `OnUpdateReady`・各ファイルの処理の後・置き換えの前に、それまでのイベントの台詞の再生が終わるまで待ってから次へ進む（台詞を返すと更新はその再生が終わるまで進まない）
3. 対象ごとに一連のイベントが揃う（シェル・バルーンもそれぞれの読み直しの後に `OnUpdateOtherComplete`）。まとめて更新するときの順はゴースト → シェル → バルーン（→ 推奨バルーン）
4. 総括は全ての更新の後に 1 回 `OnUpdateResultEx`、それが台本を返さなかったときに限り続けて `OnUpdateResult`
5. 自動更新（理由 `auto`）では `OnUpdateBegin` は更新ファイルの有無が分かった時点（`OnUpdateReady` の直前）。更新なし・確認前の失敗ではイベント 0

## Desired Outcome

- 正典の発生順序どおりの順・間合いでイベントが届く（1〜4。5 は自動更新を持つかの裁定しだい）
- 既知の制限「`OnUpdateResult` が `OnUpdateComplete` の台詞の終わりを待たない」が解ける（2 と同根）
- `doc/COMPAT_ARCHITECTURE.md` §8 の該当行と網羅台帳の備考が新しい流れに揃う

## Approach

要件の段で決める（候補）:

- **kanade に「今の台詞が終わってから次へ」の口を足す**（2 と既知の制限の根）。形は `pending_change` と同じ「控え 1 枠」か、送出の側で台詞の終わりの知らせ（新しい `KanadeNotice`）を受けて背景スレッドを進めるか。記憶 areka-host32-is-bolt-on…（kanade の改変は疎結合化の方向でのみ）と、完了 `kanade` の運行表を崩さないことが制約
- **エンジン `areka-update` の MD5 の取り直し**（1）。取り直しの上限と、取り直しのたびに進捗 `Progress` をどう出すか（`OnUpdate.OnDownloadBegin` を同じファイルで再び出す）
- **対象ごとの読み直し**（3）。シェル・バルーンの読み直しの口（完了 or 進行中の `shell-balloon-switch` の入口）を使うか、最後に 1 回の読み直しのまま `OnUpdateOtherComplete` だけを後送りにするか
- **総括の 2 段送り**（4）。`OnUpdateResultEx` の返事を見てから `OnUpdateResult` を送る（`RaiseOutcome` の返事を待つ）

## Scope

- **In**: 上の差 1〜4 を正典どおりにすること・そのための kanade の口・エンジンの取り直し・`network-update` の手続きと窓口の順の組み替え・テストの書き換え（消さない）・COMPAT §8 と台帳の備考・実機（emo2 の配布サイトで差分を置いて 1 周）
- **Out**: 自動更新そのもの（差 5 は自動更新を足す spec が持つ。本仕様で足すかは要件で決める）・更新オプション（`checkonly` など・`OnUpdateCheck*`）・`other_homeurl_override`・推奨バルーンの更新（持つかは要件で決める）

## Boundary Candidates

- kanade の「台詞の後に進む／送る」口（運行表への追加・背景スレッドへの知らせ）
- エンジンの MD5 の取り直し（`crates/areka-update`）
- 手続き・窓口の順の組み替え（`crates/areka/src/update/`）

## Out of Boundary

- 切替の入口の判定・`switch_to_default`（完了 `ghost-shell-balloon-switch`）
- インストールの手続き（`install/`）
- 網羅台帳の生成器

## Upstream / Downstream

- **Upstream**: 完了 `areka-P0-network-update`・完了 `areka-P0-update-engine`・完了 `areka-P0-kanade`・`shell-balloon-switch`（シェル・バルーンの読み直しの口を使うなら）
- **Downstream**: 自動更新を足す spec（未起票）・`mcp-kanade-tools`（台詞の終わりの知らせを使える）

## Existing Spec Touchpoints

- **Extends**: 完了 `areka-P0-network-update`（流れ）・完了 `areka-P0-update-engine`（取り直し）
- **Adjacent**: `shell-balloon-switch`（切替の入口・シェル・バルーンの読み直し）・`coverage-roadmap-refresh`（台帳の数）

## Constraints

- 意味論は ukadoc から輸入する（SSP の実測で合わせない・記憶 no-ssp-measurement-import-semantics-from-ukadoc）。根拠は上のコミットの `caption_updateorder` 節
- 1 フレーム遅らせる解は取らない（記憶 no-frame-delay-fixes-change-the-state-shape）
- 決定論のテストで順を固定する（完了 `network-update` の `worker_path_tests.rs`・`desk_reload_tests.rs` の形を引き継ぐ）
- 規模の見立て: M（10〜15 タスク）。Fable で要件定義（kanade の口の形・読み直しの単位の裁定がある）
