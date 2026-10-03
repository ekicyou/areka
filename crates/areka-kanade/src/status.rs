//! `Status` 実行状態語彙と、その送出契約（正典順・カンマ連結・空集合→省略）の正本。
//!
//! ukadoc `Status [SSP拡張]` の実行状態語彙（全10状態）を第一級の型として保持し、
//! アクティブ集合 → wire 文字列（`None` ⇔ ヘッダ行を出さない）の写像を**唯一**所有する。
//!
//! ## 依存規律
//!
//! 本モジュールは `std` のみに依存する（host32 型・areka-actor 型に非依存）。`talk.rs` と
//! 同じ**葉の契約型**の規律であり、将来の契約クレート切り出しは機械的移動で済む（DD-1）。
//!
//! ## 語彙と下位書式（DD-IT-9）
//!
//! パラメータ付き状態の下位書式は ukadoc 正典どおり内部区切りに `/` を用いる
//! （`opening(communicate/input/teach/dialog)`／`balloon(0=2/1=0)`）。トップレベルの状態
//! 連結は `,` であり、内部 `/` と分離されているため `,` 分割が曖昧にならない。実 SSP 2.3.86 は
//! `balloon(0=2,1=0)` を送る差異があるが、areka は ukadoc 正典の `/` を送る
//! （`areka-P0-status-execution-states` で `balloon` を実導出・SSP 実測主義は取らない）。

/// ukadoc `Status [SSP拡張]` の実行状態語彙（正典全10状態を第一級保持）。
///
/// 実導出するのは `Talking`（idle-talk）・`Choosing`（choice-select-events）と、
/// `NoUserBreak`・`Online`・`Balloon`（status-execution-states）の 5 状態。
/// 残 5 状態は語彙に留め、非アクティブへ縮退する（status-execution-states Req6.1/6.2）。
/// variant の宣言順は正典の語彙定義順（Data Model の正典順）そのものである。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionState {
    Talking,
    Choosing,
    Minimizing,
    Induction,
    Passive,
    TimeCritical,
    NoUserBreak,
    Online,
    /// `opening(種類)` — 種類は `/` 区切りで列挙（正典例 `opening(communicate/input/teach/dialog)`）
    Opening(OpeningKinds),
    /// `balloon(ID群)` — `charID=balloonID` を `/` 区切りで列挙（正典例 `balloon(0=2/1=0)`）
    Balloon(BalloonBindings),
}

impl ExecutionState {
    /// 正典順の序数（Data Model の語彙定義順）。集合の正典順ソート・重複判定の鍵。
    fn canonical_index(&self) -> u8 {
        match self {
            ExecutionState::Talking => 0,
            ExecutionState::Choosing => 1,
            ExecutionState::Minimizing => 2,
            ExecutionState::Induction => 3,
            ExecutionState::Passive => 4,
            ExecutionState::TimeCritical => 5,
            ExecutionState::NoUserBreak => 6,
            ExecutionState::Online => 7,
            ExecutionState::Opening(_) => 8,
            ExecutionState::Balloon(_) => 9,
        }
    }

    /// 単一状態の wire トークン。パラメータ付き状態は下位書式（内部 `/`・DD-IT-9）を含む。
    fn render_token(&self) -> String {
        match self {
            ExecutionState::Talking => "talking".to_string(),
            ExecutionState::Choosing => "choosing".to_string(),
            ExecutionState::Minimizing => "minimizing".to_string(),
            ExecutionState::Induction => "induction".to_string(),
            ExecutionState::Passive => "passive".to_string(),
            ExecutionState::TimeCritical => "timecritical".to_string(),
            ExecutionState::NoUserBreak => "nouserbreak".to_string(),
            ExecutionState::Online => "online".to_string(),
            ExecutionState::Opening(kinds) => format!("opening({})", kinds.render_inner()),
            ExecutionState::Balloon(bindings) => format!("balloon({})", bindings.render_inner()),
        }
    }
}

/// `opening(種類)` の種類集合（`/` 区切り列挙）。空集合は `Opening` 自体を非アクティブとする。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpeningKinds(Vec<OpeningKind>);

impl OpeningKinds {
    /// 種類列から構成する。
    pub fn new(kinds: Vec<OpeningKind>) -> Self {
        OpeningKinds(kinds)
    }

    /// 種類を `/` 区切りで連結した下位書式（DD-IT-9）。
    fn render_inner(&self) -> String {
        self.0
            .iter()
            .map(OpeningKind::token)
            .collect::<Vec<_>>()
            .join("/")
    }
}

/// 正典が列挙する入力ボックス等の種類。ukadoc の例示が閉集合である保証が無いため拡張シームを持つ。
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum OpeningKind {
    Communicate,
    Input,
    Teach,
    Dialog,
}

impl OpeningKind {
    fn token(&self) -> &'static str {
        match self {
            OpeningKind::Communicate => "communicate",
            OpeningKind::Input => "input",
            OpeningKind::Teach => "teach",
            OpeningKind::Dialog => "dialog",
        }
    }
}

/// `balloon(ID群)` の束縛集合（`/` 区切り列挙）。空集合は `Balloon` 自体を非アクティブとする。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BalloonBindings(Vec<BalloonBinding>);

impl BalloonBindings {
    /// 束縛列から構成する。`character_id` 昇順へ安定整列し、同じ `character_id` は先頭だけ残す
    /// （不変条件「昇順・重複なし」を構成の時点で保証する・`ExecutionStatus::from_states` と同じ流儀）。
    pub fn new(mut bindings: Vec<BalloonBinding>) -> Self {
        bindings.sort_by_key(|b| b.character_id);
        bindings.dedup_by_key(|b| b.character_id);
        BalloonBindings(bindings)
    }

    /// `charID=balloonID` を `/` 区切りで連結した下位書式（DD-IT-9）。
    fn render_inner(&self) -> String {
        self.0
            .iter()
            .map(|b| format!("{}={}", b.character_id, b.balloon_id))
            .collect::<Vec<_>>()
            .join("/")
    }
}

/// `charID=balloonID` の 1 対。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BalloonBinding {
    pub character_id: u32,
    pub balloon_id: u32,
}

/// `Status` ヘッダの値＝アクティブな実行状態の集合（正典順・重複なし）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionStatus {
    states: Vec<ExecutionState>,
}

impl ExecutionStatus {
    /// 状態列を正典順へ整列し、状態種ごとに重複を除いて構成する。
    /// 不変条件（正典順・重複なし）を**構成の時点で**保証する唯一の内部コンストラクタ。
    fn from_states(mut states: Vec<ExecutionState>) -> ExecutionStatus {
        // 安定ソート＋種別鍵の隣接重複除去で「正典順・各状態種は高々1つ」を保証する。
        states.sort_by_key(ExecutionState::canonical_index);
        states.dedup_by_key(|s| s.canonical_index());
        ExecutionStatus { states }
    }

    /// 単一の導出表（正典順の10行）。5 行を実導出し、残 5 行は非アクティブ確定＋持ち主の注記。
    /// スナップショットのみに依存する純関数（時刻・IO・グローバル状態を読まない）。
    pub fn derive(snapshot: &ExecutionSnapshot) -> ExecutionStatus {
        let mut states: Vec<ExecutionState> = Vec::new();

        // 導出表（正典順の10行）。行の存在自体が語彙保持の表明である（Req2.5）。
        // 非アクティブ行は Reference1/Reference2 の固定 "0" と同型の縮退であり、源着地時は
        // `ExecutionSnapshot` へフィールドを 1 本足して当該行を差し替える（送出契約は不変）。
        //
        //  0. talking      ← snapshot.talk_active（M1 実導出・Req2.4/2.7）
        if snapshot.talk_active {
            states.push(ExecutionState::Talking);
        }
        //  1. choosing     ← snapshot.choice_active（実導出・areka-P0-choice-select-events Req6.1/6.2）
        if snapshot.choice_active {
            states.push(ExecutionState::Choosing);
        }
        //  2. minimizing   ← SEAM(Req6.1/6.3): 出どころ未着地。持ち主＝要件 6.3 の宛先。非アクティブ確定。
        //  3. induction    ← SEAM(Req6.1/6.3): 出どころ未着地。持ち主＝要件 6.3 の宛先。非アクティブ確定。
        //  4. passive      ← SEAM(Req6.1/6.3): 出どころ未着地。持ち主＝要件 6.3 の宛先。非アクティブ確定。
        //  5. timecritical ← SEAM(Req6.1/6.3): 出どころ未着地。持ち主＝要件 6.3 の宛先。非アクティブ確定。
        //  6. nouserbreak  ← snapshot.no_user_break（実導出・areka-P0-status-execution-states Req3）
        if snapshot.no_user_break {
            states.push(ExecutionState::NoUserBreak);
        }
        //  7. online       ← snapshot.online（実導出・areka-P0-status-execution-states Req2）
        if snapshot.online {
            states.push(ExecutionState::Online);
        }
        //  8. opening      ← SEAM(Req6.1/6.3): 出どころ未着地。持ち主＝要件 6.3 の宛先。非アクティブ確定。
        //  9. balloon      ← snapshot.balloons（実導出・Req4。空の組は行を出さない＝Req4.4）
        if !snapshot.balloons.is_empty() {
            states.push(ExecutionState::Balloon(BalloonBindings::new(
                snapshot.balloons.clone(),
            )));
        }

        ExecutionStatus::from_states(states)
    }

    /// wire 値への写像。**`None` ⇔ 空集合 ⇔ `Status` ヘッダ行を出さない**（Req2.3・DD-IT-5）。
    /// `Some` のときは正典順でカンマ連結した値であり、空文字列は決して返さない。
    pub fn render(&self) -> Option<String> {
        if self.states.is_empty() {
            return None;
        }
        Some(
            self.states
                .iter()
                .map(ExecutionState::render_token)
                .collect::<Vec<_>>()
                .join(","),
        )
    }
}

/// リクエスト送出時点のゴースト実行状態スナップショット。
/// `Status` 実行状態集合と OnSecondChange の Reference 値の**共通の源**であり、
/// 両者の不整合（例: Ref3="1" かつ `Status: talking`）を構造的に排除する（DD-IT-3）。
///
/// `balloons` が `Vec` を持つため `Copy` は持たない（`Clone` で複製する）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionSnapshot {
    /// トーク再生中か。源＝運行状態 `Phase::Steady{talk: Some(_)}`（Req2.4）。
    /// `Status: talking`（Req2.4/2.7）と Reference3（Req1.4/1.5）の双方を駆動する。
    pub talk_active: bool,
    /// 選択待ちが継続中か。源＝kanade の選択帳簿 `State.choice`——その 3 段フェーズ
    /// （`Waiting`／`Cascading`／`TimeoutInFlight`）の**いずれでも継続中**である
    /// （`Cascading`／`TimeoutInFlight` は SHIORI 応答待ちであって選択待ちの終了ではない）。
    /// `Status: choosing`（Req6.1/6.2）を駆動する。talk slot の占有は選択待ち中も継続する
    /// ため、`talk_active` と同時に真になり複合値 `talking,choosing` を成す（裁定 6）。
    pub choice_active: bool,
    /// 再生中のトークが中断の無効化モード中か（`\![enter,nouserbreakmode]`〜`\![leave,…]`）。
    /// 源＝kanade の写し（UI の中断を禁じる旗）。再生中でないときに真にしないのは作り手
    /// （`State`）の責務であり、導出表は欄をそのまま写す。`Status: nouserbreak` を駆動する。
    pub no_user_break: bool,
    /// ネットワーク通信中か（更新の手続き・URL からのダウンロード）。源＝プロセスに 1 つの
    /// 通信中の数を殻が写した値。`Status: online` を駆動する。
    pub online: bool,
    /// 見えているバルーンの組（`charID=balloonID`）。空なら `balloon` を出さない。
    /// 源＝UI のバルーン可視性の相が差分で送る写し。`Status: balloon(…)` を駆動する。
    pub balloons: Vec<BalloonBinding>,
    // SEAM(Req1.6): 見切れ／重なりの実測供給時に `offscreen`／`overlapping` を追加する。
    //   源＝窓 geometry（UI スレッド）・運搬＝Tick 付帯。所有＝将来増分（本 spec 外）。
    // SEAM(Req6.3): minimizing/induction/passive/timecritical/opening は出どころが着地したら
    //   フィールドを 1 本追加し、導出表の該当行を差し替える（持ち主＝要件 6.3 の宛先）。
    //
    // NOTE(シームの実体＝「フィールド 1 本」では閉じない): 源が Phase の外にある状態は、
    //   欄に加えて**作り手**（`schedule/mod.rs` の `State::snapshot`／`State::snapshot_without_talk`
    //   ——本番のスナップショットはこの 2 つからしか作らない）へ材料を届ける運搬も要る。
    //   `choice_active`（源＝選択帳簿）と、写し（`ExternalStates`）から作る `no_user_break`・
    //   `online`・`balloons` がその実例である。不変を保証するのは **wire 送出契約**
    //   （カンマ連結書式・ヘッダ位置・空集合→行省略・Reference 連番）であって内部シグネチャではない。
}

impl ExecutionSnapshot {
    /// 全実行状態が非アクティブなスナップショット（テストと構造体リテラルの既定値の継ぎ足し用）。
    /// 本番の送出は `State::snapshot`／`State::snapshot_without_talk` から作り、これを直接渡さない。
    pub const INACTIVE: ExecutionSnapshot = ExecutionSnapshot {
        talk_active: false,
        choice_active: false,
        no_user_break: false,
        online: false,
        balloons: Vec::new(),
    };
}

/// 外から届いた実行状態の写し（kanade の `State` が持つ・ゴーストごとに新品）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExternalStates {
    /// UI の中断を禁じる旗の写し（UI の旗を遅れて追う鏡。トークの境界で勝手に下ろさない）。
    pub no_user_break: bool,
    /// 通信中か（殻が `online::PROCESS` から毎メッセージ写す）。
    pub online: bool,
    /// 見えているバルーンの組（UI の可視性の相が差分で送る・`character_id` 昇順）。
    pub balloons: Vec<BalloonBinding>,
}

/// UI から kanade への知らせ（`KanadeMsg::ExecutionState` の中身）。online は殻が読むので無い。
/// 値は増分ではなく「今の状態」そのものであり、落ちても次の変化で正しくなる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionStateUpdate {
    NoUserBreak(bool),
    Balloons(Vec<BalloonBinding>),
}

impl ExternalStates {
    /// 知らせを写しへ反映する（純関数・返り値は写しが変わったか）。
    pub fn apply(&mut self, update: ExecutionStateUpdate) -> bool {
        match update {
            ExecutionStateUpdate::NoUserBreak(value) => {
                std::mem::replace(&mut self.no_user_break, value) != value
            }
            ExecutionStateUpdate::Balloons(balloons) => {
                if self.balloons == balloons {
                    return false;
                }
                self.balloons = balloons;
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unit Test #1（Req2.1）: 全10 variant が正典トークンへ写り、パラメータ付き状態の
    /// 下位書式が `opening(communicate/input/teach/dialog)`／`balloon(0=2/1=0)` を表現できる。
    #[test]
    fn every_state_renders_to_its_canonical_token() {
        assert_eq!(ExecutionState::Talking.render_token(), "talking");
        assert_eq!(ExecutionState::Choosing.render_token(), "choosing");
        assert_eq!(ExecutionState::Minimizing.render_token(), "minimizing");
        assert_eq!(ExecutionState::Induction.render_token(), "induction");
        assert_eq!(ExecutionState::Passive.render_token(), "passive");
        assert_eq!(ExecutionState::TimeCritical.render_token(), "timecritical");
        assert_eq!(ExecutionState::NoUserBreak.render_token(), "nouserbreak");
        assert_eq!(ExecutionState::Online.render_token(), "online");

        // opening(種類) — 種類は `/` 区切り列挙（DD-IT-9: 内部は `/`・トップレベルは `,`）
        let opening = ExecutionState::Opening(OpeningKinds::new(vec![
            OpeningKind::Communicate,
            OpeningKind::Input,
            OpeningKind::Teach,
            OpeningKind::Dialog,
        ]));
        assert_eq!(
            opening.render_token(),
            "opening(communicate/input/teach/dialog)"
        );

        // balloon(ID群) — `charID=balloonID` を `/` 区切り列挙（DD-IT-9）
        let balloon = ExecutionState::Balloon(BalloonBindings::new(vec![
            BalloonBinding {
                character_id: 0,
                balloon_id: 2,
            },
            BalloonBinding {
                character_id: 1,
                balloon_id: 0,
            },
        ]));
        assert_eq!(balloon.render_token(), "balloon(0=2/1=0)");
    }

    /// Unit Test #2（Req2.2/2.3）: 複数状態が正典順でカンマ連結される
    /// （`talking,choosing,balloon(0=2/1=0)`）／空集合 → `render() == None`。
    #[test]
    fn active_states_render_in_canonical_order_and_empty_omits_the_line() {
        // 非正典順で与えても、構成が正典順を保証する（不変条件・DD Data Model 順）。
        let status = ExecutionStatus::from_states(vec![
            ExecutionState::Balloon(BalloonBindings::new(vec![
                BalloonBinding {
                    character_id: 0,
                    balloon_id: 2,
                },
                BalloonBinding {
                    character_id: 1,
                    balloon_id: 0,
                },
            ])),
            ExecutionState::Talking,
            ExecutionState::Choosing,
        ]);
        assert_eq!(
            status.render(),
            Some("talking,choosing,balloon(0=2/1=0)".to_string())
        );

        // 空集合 → ヘッダ行そのものを省略（DD-IT-5）。空値 `Status:` ではなく `None`。
        let empty = ExecutionStatus::from_states(vec![]);
        assert_eq!(empty.render(), None);
    }

    /// Unit Test #3（Req2.4/2.5/2.7）: `derive` の talk 軸（`choice_active=false` 断面）を網羅。
    /// `talk_active=false` → 空（choosing 以外の非導出状態は決して現れない）／`true` → `[Talking]` のみ。
    /// choice 軸を含む全入力空間は [`derive_covers_the_full_two_bool_input_space`] が固定する。
    #[test]
    fn derive_covers_the_entire_bool_input_space() {
        // talk_active=false → 空集合（choosing は choice_active=false・残8状態は非導出）→ 行省略（Req2.3）。
        let idle = ExecutionStatus::derive(&ExecutionSnapshot::INACTIVE);
        assert_eq!(idle.render(), None);

        // talk_active=true → `[Talking]` のみ（Req2.4/2.7）。
        let talking = ExecutionStatus::derive(&ExecutionSnapshot {
            talk_active: true,
            choice_active: false,
            ..ExecutionSnapshot::INACTIVE
        });
        assert_eq!(talking.render(), Some("talking".to_string()));

        // design 明記の Postconditions を明示的に固定する。
        assert_eq!(
            ExecutionStatus::derive(&ExecutionSnapshot::INACTIVE).render(),
            None
        );
        assert_eq!(
            ExecutionStatus::derive(&ExecutionSnapshot {
                talk_active: true,
                choice_active: false,
                ..ExecutionSnapshot::INACTIVE
            })
            .render(),
            Some("talking".to_string())
        );
    }

    /// Unit Test #4（Req6.1/6.2/6.3・C5）: `derive` の全入力空間（bool 2 本）を網羅する。
    ///
    /// choosing 行の実導出後も**送出契約は無改変**である——複合値は既存 `canonical_index` の
    /// 順序どおり `talking,choosing` で連結され（Req6.3）、両方非アクティブなら
    /// `render() == None`＝ヘッダ行そのものを省略する（Req2.3 の規律を choice 軸が壊さない）。
    #[test]
    fn derive_covers_the_full_two_bool_input_space() {
        let cases: [(bool, bool, Option<&str>); 4] = [
            (false, false, None),
            (true, false, Some("talking")),
            (false, true, Some("choosing")),
            (true, true, Some("talking,choosing")),
        ];
        for (talk_active, choice_active, expected) in cases {
            let rendered = ExecutionStatus::derive(&ExecutionSnapshot {
                talk_active,
                choice_active,
                ..ExecutionSnapshot::INACTIVE
            })
            .render();
            assert_eq!(
                rendered.as_deref(),
                expected,
                "talk_active={talk_active} choice_active={choice_active} の導出が正典と不一致"
            );
        }
    }

    fn binding(character_id: u32, balloon_id: u32) -> BalloonBinding {
        BalloonBinding {
            character_id,
            balloon_id,
        }
    }

    /// 要件 1.1/4.3/4.4: nouserbreak・online・balloon の 3 行が欄をそのまま写し、
    /// 正典順で連結され、空の組では `balloon` を出さない（全組み合わせは兄弟テストが固定する）。
    #[test]
    fn derive_maps_nouserbreak_online_and_balloon_rows() {
        let all = ExecutionSnapshot {
            talk_active: true,
            choice_active: true,
            no_user_break: true,
            online: true,
            balloons: vec![binding(0, 2), binding(1, 0)],
        };
        assert_eq!(
            ExecutionStatus::derive(&all).render().as_deref(),
            Some("talking,choosing,nouserbreak,online,balloon(0=2/1=0)")
        );
        let online_only = ExecutionSnapshot {
            online: true,
            ..ExecutionSnapshot::INACTIVE
        };
        assert_eq!(
            ExecutionStatus::derive(&online_only).render().as_deref(),
            Some("online")
        );
        // 空の組は `balloon()` を出さない（要件 4.4）。
        let talking_no_balloon = ExecutionSnapshot {
            talk_active: true,
            balloons: Vec::new(),
            ..ExecutionSnapshot::INACTIVE
        };
        assert_eq!(
            ExecutionStatus::derive(&talking_no_balloon)
                .render()
                .as_deref(),
            Some("talking")
        );
    }

    /// 要件 4.3/1.3: 組は構成の時点で `character_id` 昇順・重複なし（先頭を残す）になる。
    #[test]
    fn balloon_bindings_sort_ascending_and_drop_duplicate_characters() {
        let bindings = BalloonBindings::new(vec![binding(1, 0), binding(0, 2), binding(1, 5)]);
        assert_eq!(bindings.render_inner(), "0=2/1=0");
        let snapshot = ExecutionSnapshot {
            balloons: vec![binding(1, 0), binding(0, 2)],
            ..ExecutionSnapshot::INACTIVE
        };
        assert_eq!(
            ExecutionStatus::derive(&snapshot).render().as_deref(),
            Some("balloon(0=2/1=0)")
        );
    }

    /// 写しへの知らせの反映は「変わったか」を返し、同じ値の知らせは変化なしとする。
    #[test]
    fn external_states_apply_reports_whether_the_copy_changed() {
        let mut external = ExternalStates::default();
        assert!(external.apply(ExecutionStateUpdate::NoUserBreak(true)));
        assert!(!external.apply(ExecutionStateUpdate::NoUserBreak(true)));
        assert!(external.no_user_break);
        assert!(external.apply(ExecutionStateUpdate::Balloons(vec![binding(0, 0)])));
        assert!(!external.apply(ExecutionStateUpdate::Balloons(vec![binding(0, 0)])));
        assert!(external.apply(ExecutionStateUpdate::Balloons(Vec::new())));
        assert!(external.apply(ExecutionStateUpdate::NoUserBreak(false)));
        assert_eq!(external, ExternalStates::default());
    }

    /// C5: `INACTIVE` は**全ての源が false**（choosing の源を足しても非アクティブのまま）。
    /// boot 系列・close 系列・ForceQuit 後がこの定数で送出する以上、源の増設で
    /// 既定値が汚れないことを固定する。
    #[test]
    fn inactive_snapshot_has_every_source_false() {
        // 網羅的な構造体リテラルとの比較で固定する——源が 1 本増えたときは本行が
        // コンパイルエラーになり、既定値の判断が必ず要求される。
        assert_eq!(
            ExecutionSnapshot::INACTIVE,
            ExecutionSnapshot {
                talk_active: false,
                choice_active: false,
                no_user_break: false,
                online: false,
                balloons: Vec::new(),
            }
        );
        assert_eq!(
            ExecutionStatus::derive(&ExecutionSnapshot::INACTIVE).render(),
            None
        );
    }
}

#[cfg(test)]
#[path = "status_derive_tests.rs"]
mod derive_tests;
