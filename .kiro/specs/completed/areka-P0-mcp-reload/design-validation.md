# 設計バリデーション: areka-P0-mcp-reload

> 2026-10-18・本ブランチ（`a5d363ff` design 生成の直後）。入力: requirements.md（13 要件・確定）・design.md・research.md（§9〜§11）・steering（product／tech／structure／roadmap）。設計の主張はコードで照合した（照合した定義は本文に「ファイル＋関数名」で記す・行番号は本日時点）。非対話で書いた（質問はしていない）。

## レビュー要約

設計は「読み直し＝切替先が今のものである切替の特別な場合」と一般化し、既存の 2 つの載せ替えの道（`request_ghost_switch_with`・`request_skin_switch_with`）の**受理の後半を切り出して直接呼ぶ**形に閉じている。コードで照合した限り、この切り出し線は実在し（両関数とも「目録の解決の後」から末尾の `insert_non_send` までが副作用の塊）、`wire_emo2_boot` の窓の側／実行系の側の分け目（手順 1〜4 で作る送出端はすべて `Clone`、`Emo2Wiring` が抱える受信端は 4 本、boot は手順 5）も実物と一致する。ウェーブの禁止ファイル（kanade `msg.rs`・`actor.rs`・`schedule/mod.rs`・`schedule/change.rs`・`crates/areka-mcp/**`）は変更一覧に無い。ただし、要件 4 の中核である `switch_to` の分かれ目の**置き場所の記述が実コードの手順の順と食い違う**点と、`schedule/events.rs` が並走 spec の触るファイルに載っている点は、実装前に設計の文面で確定しておく必要がある。

## 重大な問題（最大 3）

### 🔴 Critical Issue 1: `switch_to` の shiori の分かれ目が `take_down` の後に置かれている

- **Concern**: design「GhostSwitch の口」は「⑴ `write_switch_drop` の後、`if reload::wants_runtime_only(world) { rebuild_runtime_in_place … }`」と書く。しかし実物の `ghost_switch.rs::switch_to` は `switch_copy::boot_dir` → **`take_down(world)`** → `write_switch_drop` → `close_windows_for_restart` → `boot_into` の順で、`take_down` は `GhostSlot` の単位を**丸ごと** `shutdown(System)` する（seriko の join・loop ticker の Close・`seriko_sink` の drop まで）。文面どおりに実装すると、shiori の読み直しでも窓の側（seriko・ticker）が降り、`GhostSlot` は空になり、`rebuild_runtime_in_place` が `split_runtime` する相手が無い。design 自身の Flow 3・不変条件 ⑶「shiori の読み直しでは `GhostSlot` は空にならない」と矛盾する。
- **Impact**: 要件 4.2（窓とシェル・バルーンを残す）・12.5 が満たせない。`write_switch_drop` の前提「実行系が 1 つも動いていない間だけ UI が App スコープへ直接書く」も、古い実行系を降ろす前に書けば崩れる。
- **Suggestion**: 分かれ目を **`take_down` の手前**（`switch_copy::boot_dir` の直後）に置き、`rebuild_runtime_in_place` の中の順を「`split_runtime` → 古い `GhostRuntime::shutdown(System)`（所要 ms の記録）→ `write_switch_drop` → `boot_runtime` → `attach_runtime`・`rewire_runtime_consumers`」と明記する（`take_down`・`close_windows_for_restart`・`run_overwrite_between` は呼ばない）。失敗時は実行系の無い単位を `GhostSlot` に戻したまま `switch_to_default` へ（`take_down` は `ghost: None` を扱える＝`shutdown_impl` ②が `if let Some`）。
- **Traceability**: 要件 4.1・4.2・4.4・12.5
- **Evidence**: design.md「GhostSwitch の口」の箇条 ⑴、「Flow 3」、「Data Models › 不変条件 ⑶」／コード `crates/areka/src/emo2_boot/ghost_switch.rs::switch_to`・`take_down`、`crates/areka/src/ghost_session.rs::shutdown_impl`

### 🔴 Critical Issue 2: `schedule/events.rs` は `mcp-kanade-tools` の触るファイルに載っている（「並走 spec の持ち物 0」は不正確）

- **Concern**: design「13.1 の補正」は `crates/areka-kanade/src/schedule/events.rs` に `ALLOWED_EVENT_IDS` の `"OnDestroy"` 1 行を足し、「ほかの並走 spec の持ち物に触る箇所は 0」と結ぶ。しかし `.kiro/specs/areka-P0-mcp-kanade-tools/brief.md`「触るファイル（並走の照合用）」は残る本 spec の一覧に **`schedule/events.rs`（迂回の印）** を挙げている。さらに `events_tests.rs` の `allowed_event_ids_are_exactly_the_fifty_four_and_exclude_ontalk_onhour` は表の**全語を逐語で固定し、テスト名に件数を含む**ので、両 spec が表に語を足せば定数の末尾とテスト名・配列の両方で競合する。
- **Impact**: 要件 13.3 の禁止（`msg.rs` 等）には触れないので致命ではないが、13.1「ほかを触る要が出たら止めて報告」の対象であり、後着の PR がほぼ確実にマージ競合する。設計の「0」の主張のまま承認すると、ウェーブの調整が抜ける。
- **Suggestion**: 設計ディスカッションで次のどれかを決めて design に書く: ⒜ 本 spec が `"OnDestroy"` を **`"OnClose"` の直後**（末尾でない位置）に ukadoc の注釈つきで足し、テストは配列へ 1 語足すだけにして**名前は変えない**（件数の語は本文の注釈へ移す）・後着が rebase する、⒝ `mcp-kanade-tools` 側に `OnDestroy` の 1 行を委ねる（本 spec は着地を待つ）。「並走 spec の持ち物 0」の文は「`events.rs` 1 行だけ重なる・調整済み」に改める。
- **Traceability**: 要件 13.1・13.3・4.1
- **Evidence**: design.md「要件 13.1 の一覧の補正」の表と末尾の段落／`.kiro/specs/areka-P0-mcp-kanade-tools/brief.md`「触るファイル」／`crates/areka-kanade/src/schedule/events_tests.rs`

### 🔴 Critical Issue 3: 既存の入口は `ReloadInFlight` を見ない——印を読む述語は「段」で判定する契約を明文化する

- **Concern**: `request_ghost_switch_with`・`request_skin_switch_with`（既存・不変）は `ReloadInFlight` を見ない。設計はそれを前提に「`AwaitingDestroy` の間と skin の列の間は切替が通りうる」と認めているが、その間に通った**本物の**切替に対して、差し替えの相が読む `skin_is_reload(world)` や `switch_to` が読む `boot_origin_override(world)`・`wants_runtime_only(world)` が「印が在る」だけで真を返すと、利用者の `\![change,shell,…]` が `OnShellChanged`／`LastShell` 無しで済んでしまう（例: shiori の読み直しが `AwaitingDestroy` で待つ 1 往復の間に、メニューからシェルを切り替えた場合）。design の各述語の説明は「印が shiori の読み直しなら」「印が shell／balloon／descript の読み直しなら」と書くが、段（`ReloadStage`）と `SkinSwitchInFlight.target` の対応を取る条件が書かれていない。
- **Impact**: 要件 5.3・6.3 の逆（既存の切替の知らせと記憶を壊す）を、読み直しが同時に居る稀な場面で起こす。既存のテストは印の無い世界で走るので檻に掛からない。
- **Suggestion**: 3 つの述語の契約を design に明記する: `skin_is_reload` は「`stage == Skins` **かつ** 進行中の `SkinSwitchInFlight` が窓口自身の `accept_skin` で立てたもの（窓口が `SkinStep` を始めるときに記録した `kind`＋`target.dir` と一致）」、`boot_origin_override`／`wants_runtime_only` は「`stage == Switching` かつ `SwitchInFlight` が窓口の `accept_switch` で立てたもの」で真。簡単な実装は、窓口が受理の直後に `ReloadInFlight` へ「自分が立てた予約の鍵（`SkinKind`＋dir／`SwitchTarget.dir`）」を控え、述語はそれを照合する。`reload_tests.rs`／`reload_skin_tests.rs` に「印があっても他人の予約なら偽」の檻を 1 本ずつ足す。
- **Traceability**: 要件 5.3・6.3・3.6・12.4
- **Evidence**: design.md「ReloadDesk › 既存の道からの合図」（`boot_origin_override`・`wants_runtime_only`・`skin_is_reload`）、「Data Models › 不変条件 ⑴」／コード `crates/areka/src/emo2_boot/shell_balloon_switch.rs::request_skin_switch_with`（判定 3 つ＝`SkinSwitchInFlight`・`SwitchInFlight`・`update::desk::is_busy`）

## 設計の強み

1. **既存の道の再利用が実物の切れ目に合っている。** `request_ghost_switch_with` は目録の解決（`resolve_special_name`・`resolve_switch_target`・`pin_to_current`）の後、`PrevGhost` の組み立て → `ChangeGhost` 送出 → `switch_copy::start` → `SwitchInFlight` 挿入が 1 塊で、`accept_switch` として切り出せる。`request_skin_switch_with` も `resolve_skin_target` の後の `spawn_switch_build` → `AwaitTalkGap` → `SkinSwitchInFlight` が同じ形で、`accept_skin` の切り出しは純移動で済む。判定の順・文言は変わらない（roadmap の保存義務に合う）。
2. **窓の側／実行系の側の分け目が、既にあるコードの境と一致する。** `GhostSession` は実行系（`ghost`）と seriko・ticker・`seriko_sink` を別の欄で持ち、`shutdown_impl` の ①②③ の順がそのまま「実行系だけ降ろす」に使える。`wire_emo2_boot` の 4 本の受信端（present・move・lifecycle・zorder）の送出端は `SerikoSink`・`EmoTextSink`（`impl Clone` あり）・`TalkClock`・`Sender` ですべて複製でき、`CueSinkSources` は新しい線を要しない。kanade 側は `BootOrigin` の網羅 `match` が `schedule/boot.rs::boot_root` の 1 か所（`events.rs::on_boot` は `if let`）なので、変種を足しても既存の 4 由来のテスト（12.3）は触らずに済む。`log_history::last_id()` も既に在る（`#[allow(dead_code)]`）。

## 最終判定

**GO**（上の 3 点を設計ディスカッションで文面に反映することを条件に、実装へ進んでよい）。

- **根拠**: アーキテクチャの食い違いは無く、ウェーブの禁止ファイルにも触れない。3 つの問題はいずれも設計の**記述の精度**（手順の順・重なりの申告・述語の契約）で、骨格を変えずに design.md の数行で直せる。要件 1〜13 は追跡表で全部に部品が当たっており、決定論テストの計画も判断の分岐に絞られている。
- **次の手順**: ⑴ `/kiro-design-discussion areka-P0-mcp-reload` で Issue 1〜3 を議題に載せる（Issue 2 は `mcp-kanade-tools` との調整の答えが要る）。⑵ 反映後に `/kiro-spec-tasks areka-P0-mcp-reload`。最初のタスク群は design のとおり純移動（`consumer_ledger_tests.rs`・`special_name.rs`・`runtime_boot.rs`）から始める。

### 補足（判定を変えない観察）

- `boot_start` は今 `config` を取らない（`boot.rs::step` は持っている）＝設計どおり引数を 1 つ足すだけ。`events.rs` 冒頭の表の注釈「M1 にリロード概念なし」は実装時に直す。
- `talk_gap::begin` は Steady でなければ `Left{NotSteady}`、`pending_close` があれば同じ。shiori の読み直しの `AwaitingDestroy` はこれらを設計どおり失敗の記録で受ける。`OnDestroy` は `events::allowed_static` を通るので Issue 2 の 1 行が無いと `NotSent{NotAllowed}` になる（設計の指摘どおり）。
- `ghost_session.rs` 885 行＋約 80 行の見積もりは上限に近い。design「Risks」の `ghost_session_runtime.rs` への分離を最初から採る方が安全。
