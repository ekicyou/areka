# Brief: areka-P0-property-ipc-transport

> 起票: 2026-09-11（棚卸⑬・`areka-P0-property-query-channels` の分割 ⑵ を独立 spec に切り出し）。親 brief の「経路 6（非スクリプト同期読み）の輸送路のライブ実測と裁定（host32 IPC 運搬 or 登記付き先送り）」と「経路 5（`.ext.*` 逆方向イベント）の語彙登記」を本 spec が引き継ぐ。親 spec はスクリプト経路（1〜4）に専念し、host32 系 crate には触れない。

## Problem

里々の `get_property` 関数（Mc172-1+）等、**さくらスクリプトを経由しない同期のプロパティ読み**の輸送路が ukadoc スナップショットに記載されていない（`EXECUTE`／`GetProperty` は 0 ヒット）。areka の host32 IPC はプロパティを運ばず（ワイヤタグは Hello/Load/Request/Response/Unload の閉集合・`crates/shiori-host32-ipc/src/lib.rs`）、`IShioriHost::GetProperty/SetProperty`（`crates/shiori-abi/src/interface.rs`）の実装 3 つのうち sylphya に繋がるのは env-gate デモ経由の 1 つだけで、本番 emo2（`ShioriWiring::Helper`）には読み口が無い。里々／YAYA が同期読みを使うなら、この経路が無い限り互換が成立しない。

## Current State

2026-09-11 実測（親 brief 棚卸⑫アンカーの再測定・全命中）:
- `shiori-host32-ipc/src/lib.rs` のワイヤタグは閉集合（property 0 件・3 crate とも）。
- `shiori-abi/src/interface.rs` の `GetProperty`／`SetProperty`。実装 3 つ＝`ShioriHostSink`（`crates/areka/src/shiori_host.rs`・sylphya 接続済み・`main.rs` の env-gate デモ経由）・`InProcHost`（`crates/areka-ghost/src/shiori_inproc.rs`・非接続 `RefCell<HashMap>`）・本番 Helper（読み口なし）。
- `.ext.*`（`activeghostlist`／`pluginlist`・2.7.85）の逆方向イベント `property.get`／`property.set` は sylphya に名前だけ予約済み（`dotted.rs`）。

## Desired Outcome

輸送路が**実測で確定**している（ukadoc ライブ／SSP の挙動・里々／YAYA の実装のどれが SHIORI 側から何を呼ぶか）。確定した輸送路が host32 IPC で必要なら、`Hello…Unload` の閉集合に property 往復のタグを足し、helper→host→sylphya の読み書きが本番配線（`ShioriWiring::Helper`）で成立する。不要または不明なら、完全語彙＋縮退シーム＋本 spec への登記で先送りを閉じる（「登記付き先送り」）。

## Approach

⑴ 輸送路の調査（要件フェーズ・**ライブ正典なしに設計しない**）→ ⑵ 裁定（IPC 運搬 or 先送り）→ ⑶ 運搬なら IPC タグ追加＋`IShioriHost` の本番実装＋決定論檻（偽境界・x64）。`.ext.*` は語彙登記のみ（発火条件が多重ゴースト／プラグイン基盤＝`property-catalog-lists` の解禁と連動）。

## Scope

- **In**: 経路 6 の輸送路のライブ実測と裁定／（運搬する場合）host32 IPC のタグ追加・helper／host 両側の実装・本番 `ShioriWiring::Helper` の読み口／`.ext.*` 逆方向イベントの語彙登記。
- **Out**: スクリプト経路 1〜4（親 spec）／台帳（`sylphya-set-ledger`）／値の木（tree・catalog）／MAKOTO の第 2 helper（`makoto-dll-host`・同じ host32 crate を触るため**直列**）。

## Boundary Candidates

- 調査＋裁定（文書のみ）と、運搬の実装（コード）の 2 段。裁定が「先送り」なら 1 段目で完了する。

## Out of Boundary

- SHIORI/3 の `Charset` 交渉（`charset-canon`・先に着地）。

## Upstream / Downstream

- **Upstream**: `charset-canon`（host32-host の `shiori3.rs`／`client.rs` を先に改変・本 spec は着地後に rebase）・`host32-window-thread-pump`（`parent_window.rs`・同 crate）・完了 host32 系 4 本。
- **Downstream**: 里々／YAYA 互換・`makoto-dll-host`（host32 系 crate の後着）。

## Existing Spec Touchpoints

- **Extends**: `areka-P0-property-query-channels`（分割元・⑵ を引き継ぐ）。
- **Adjacent**: `makoto-dll-host`（host32 crate 共有＝同居不可）・`property-catalog-lists`（`.ext.*` の発火条件）。

## Constraints

- 編集集合の見込み: `crates/shiori-host32-ipc/src/lib.rs`・`crates/shiori-host32-host/src/`・`crates/shiori-host32-helper/src/`・`crates/shiori-abi/src/interface.rs`・`crates/areka/src/{shiori_host,main}.rs`・`crates/areka-ghost/src/shiori_inproc.rs`。W14 の `property-query-channels` とは `areka-ghost` crate 同居・別ファイル（channels は `prop_sink.rs`／`runtime.rs`）。
- 常時テストは x86 を避け偽境界で純 x64 決定論（記憶 prefer-x64-fake-boundary-tests-not-x86）。実機は i686 helper を先ビルド。
- **要件定義は Fable**（輸送路が未確定で、snapshot に無い一次資料の読みと 2 案の裁定が要る）。


---

## 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- 優先度 低。前提（`charset-canon`・`host32-window-thread-pump`）は完了済み。通信の札（Hello/Load/Request/Response/Unload）は変わっていない。
- **縮める**: SHIORI/3 にベースウェアへ問い合わせる正典の道は無い見込みで、答えは「登記して先送り」か「SSTP 経由」になりそうである＝**最初の 1 段（調べて裁定する・文書だけ）で終わる形にする**。輸送を作ると決まったときだけ M。`makoto-dll-host` と同じクレートを触るので同時に走らせない。`main.rs`（946 行）を触りうる。


---

## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 規模: 1 段目（調べて裁定・文書だけ）で終わるなら XS〜S（2〜4 タスク）。輸送を作ると決まったときだけ M（10〜14）。切る: なし（前回の「1 段目で終わる形」のまま）。
- 前提の状態: `charset-canon`・`host32-window-thread-pump` は着地済み。
- 崩れた前提／古くなった位置:
  - 通信の札 `crates/shiori-host32-ipc/src/lib.rs` の `MsgTag` は Hello／Load／Request／Response／Unload の 5 つのまま。`crates/shiori-abi/src/interface.rs` の `GetProperty`／`SetProperty` も変わらない。
  - `crates/areka/src/main.rs` は 957 行（前回 946・上限まで 43）。輸送を作る場合に `main.rs` を触ると上限に当たる＝結線は `ghost_session.rs` か `areka-ghost` の側へ置く。
  - 外から読む道として `mcp-get-property`（C3-⑦・SSP MCP の `get_property`）が別に作られる。これは開発者向けで、SHIORI から読む道ではない＝本 spec の裁定の材料に「同期で読みたいゴーストには何があるか」を足すだけで、役割は重ならない。
  - `.ext.*` の逆向きのイベント `property.get`／`property.set` の網羅台帳の持ち主は、まだ `property-query-channels`（`shiori.toml`）。棚卸⑬で本 spec へ移した語彙なので、1 段目の文書で台帳の持ち主も直す。
- 触るファイル（並走の照合用）:
  - 1 段目: `doc/COMPAT_ARCHITECTURE.md` §8・`doc/ukadoc-coverage/ledger/shiori.toml`（2 行の持ち主）だけ
  - 輸送を作るなら: `crates/shiori-host32-ipc/src/lib.rs`・`crates/shiori-host32-host/src/`・`crates/shiori-host32-helper/src/`・`crates/shiori-abi/src/interface.rs`・`crates/areka/src/shiori_host.rs`・`crates/areka-ghost/src/shiori_inproc.rs`
- 議題（答えで作業が変わるものだけ）: なし（1 段目の裁定そのものが本 spec の仕事）。
- 見つけた穴: 上の網羅台帳の持ち主のずれ（実害なし）。並走の照合: 1 段目は文書だけ＝どの spec とも並べられる。輸送を作るなら `makoto-dll-host`・`property-query-channels`（`SenderType` のため `shiori-host32-host` を触る）と同時に走らせない。
