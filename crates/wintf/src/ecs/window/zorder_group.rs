//! グループ単位の重なり——受け口（Resource）・観測・是正要否の純判断・記録の唯一の出口
//!
//! 「作者が並べた窓の列を、左（手前）から順に重ねる」という宣言（要件 1.1／1.2）を、
//! **`Entity` の列だけ**として受け取る層である。どの scope のどの窓かを知っているのは
//! areka の台帳であり、そちらが「存在する窓だけを手前から順に並べた列」へ射影してから
//! ここへ渡す——`wintf → areka` の import は禁止であり（既存規律 [`super::zorder_pair`]
//! の module doc）、scope 型を受け取る口も作らない。
//!
//! - [`ZOrderGroups`]: 受け口（射影済みの列・是正が要るかもしれない印・検証待ち・
//!   グループごとの連続失敗数）
//! - [`observe_group`]／[`plan_group_fixes`]: 既存の前面走査を共有した観測
//! - [`decide_group_fix`]: 是正要否の純判断（Win32 も World も触らない）
//! - `record_*`／`log_*`: 記録を実際に出す**唯一の入口**
//!
//! # 既存のペア機構との関係——土台であって、書き換える対象ではない
//!
//! 「同一スコープのバルーン窓はキャラ窓のすぐ手前」は既存のペア機構（[`super::zorder_pair`]
//! ほか 5 ファイル）が OS の owner 関係で構造保証しており、本モジュールはその 5 ファイルを
//! **1 行も編集せずに**隣へ並ぶ（要件 9.5）。共有するのは実測層の `pub(crate)` 純関数
//! （[`measure_windows_in_front`]／[`FrontScan`]）だけであり、走査を書き直さない——
//! 「Windows 上で非表示の窓を読み飛ばし、最も近い可視の隣で測る」という測り方（要件 9.3）は
//! あちらの doc に根拠ごと書かれている。中身の絵を消しているだけのバルーン窓は
//! `WS_VISIBLE` のままなので、実測上は**可視の窓として数える**（裁定済み）。
//!
//! # 既定状態＝非強制を構造で保つ（要件 6.1／6.2／6.4）
//!
//! グループが 1 本も宣言されていないとき、本モジュールは**観測すら行わない**。
//! [`plan_group_fixes`] は渡された列を舐めるだけで、列が空なら実測の口
//! （[`GroupProbe`]）を一度も呼ばず、判断も一度も走らず、計画は 0 本になる。
//! 「指令を出さないように気をつける」ではなく「出す材料がそもそも作られない」形であり、
//! これがグループ指定の無い窓どうしの前後関係を固定の規則で決めない根拠である。
//!
//! # 記録を出すマクロは本ファイル内に置くこと
//!
//! `tracing` の出力先は呼び出し元の module path が既定であり、他ファイルへ移すと
//! サインオフの grep 対象（`wintf::ecs::window::zorder_group`）が分裂する。よって
//! マクロ呼出は [`emit`] ただ 1 か所に閉じ、後続の各タスクはここに生えている
//! `record_*`／`log_*` を呼ぶ（既存ペア機構が [`super::zorder_pair_diag`] との間で
//! 敷いたのと同じ一線＝「マクロを含むか否か」）。
//!
//! **行の組立は兄弟の [`super::zorder_group_diag`] に在る**——記録タグ 3 種の定数と
//! 行を組む純関数はあちらに閉じており（マクロを 1 つも含まない）、こちらは戻り値を
//! そのまま本文にする。組立を二重に持たないための分割であり、境界は
//! 「マクロを含むか否か」の一線である。
//!
//! # 保全語彙 2 語はこのモジュールに居ない（要件 9.5）
//!
//! `[zorder-group] applied`／`[zorder-group] rejected` は本モジュールが退役しても残る
//! 語彙であり、定数・行組立・記録関数（`log_group_applied`／`log_group_rejected`）ごと
//! [`super::zorder_chain_diag`] へ**字面を 1 字も変えずに**移してある。呼び出し元は
//! `wintf::ecs::window` の再輸出を通して同じ名前で呼び続ける。

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use tracing::{debug, error, warn};
use windows::Win32::Foundation::HWND;

use super::zorder_group_diag::{fix_line, skip_line, tristate_field, verify_failed_line};
use super::zorder_pair::{FrontScan, measure_windows_in_front};

// ============================================================================
// ZOrderGroups - areka からの受け口
// ============================================================================

/// 手前から順に並べたグループ 1 本（areka の台帳からの射影）。
///
/// `members` は**存在する窓だけ**が入った列であり、まだ現れていないスコープの窓は
/// 射影の時点で落ちている（台帳側のエントリは残る＝要件 1.4）。よってこの型を見ても
/// 「宣言されたが未出現の窓」は分からない——分からなくてよい。ここが知るべきなのは
/// 「いま並べ替えられる窓が、どの順に並ぶべきか」だけである。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZOrderGroupSpec {
    /// 台帳が配ったセッション内で一意な識別子（記録の結合キー）
    pub id: u32,
    /// 手前から順の窓（先頭が最も手前）
    pub members: Vec<Entity>,
}

/// グループ機構の受け口（射影キャッシュであって正本ではない）。
///
/// 正本は areka の台帳であり、こちらは「いまの窓の並びはこう」の写しにすぎない。
/// 二重帳簿を作らないため、本 Resource には台帳へ書き戻す口を持たせない。
///
/// フィールドのうち [`Self::groups`] と [`Self::pending`] が公開なのは、射影を書き込む
/// 側（areka の drain 相）と追随トリガ（`window_pos`・バルーンの再表示）が別クレート・
/// 別モジュールに居るためである。残る 2 つ——検証待ちと連続失敗数——は維持系の内部状態で
/// あり、外から書き換えられると「出してもいない指令を検証する」「他人の失敗を数える」形が
/// 作れてしまうので非公開にし、[`ZOrderGroups::arm_verify`] ほかの口だけを開けてある。
#[derive(Resource, Default)]
pub struct ZOrderGroups {
    /// 射影済み（存在窓のみ・手前から順）
    pub groups: Vec<ZOrderGroupSpec>,
    /// 是正が必要かもしれない
    pub pending: bool,
    /// 直前巡の発行に対する検証待ち
    verify: Option<GroupVerify>,
    /// グループ ID ごとの連続 verify 失敗（頭打ち用）
    fail_streaks: HashMap<u32, u8>,
}

/// 直前巡に出した連鎖の写し（次巡の検証が「何を出したか」を復元するために持つ）。
///
/// 検証を次巡へ遅らせるのは、指令の書込（flush）が tick の後であり、発行と同じ巡の
/// 実測では必ず書込前の値になるからである——同巡の実測は証跡に使えない
/// （既存ペア機構の `record_verification` が同じ理由で 2 段階になっている）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GroupVerify {
    /// 対象グループ
    pub id: u32,
    /// 動かさなかった先頭の窓
    pub head: HWND,
    /// `chain[i]` を直前要素（`i == 0` なら [`GroupVerify::head`]）の直後へ差し込んだ
    pub chain: Vec<HWND>,
}

// SAFETY: `GroupVerify` は `HWND`（windows-rs では `*mut c_void` の newtype）を保持する
// ため自動では Send/Sync が導出されない。この手動 impl は冗長ではなく必須である
// （無いと `ZOrderGroups` が `Resource` の `Send + Sync` 要求を満たせない）。
// 健全性: 保持するのは**出した指令を書き写した値**であり、窓の所有権も解放責務も伴わない
// 不透明な識別子にすぎない。この値を実際に Win32 へ渡すのは維持系の system であり、
// Win32 を呼ぶ system は NonSend パラメータで UI スレッドに固定されている。
// `ZOrder::InsertAfter(HWND)` と同根の crate 標準の HWND 取り扱い方針。
unsafe impl Send for GroupVerify {}
unsafe impl Sync for GroupVerify {}

impl ZOrderGroups {
    /// 出した連鎖を検証待ちとして預ける（次巡に [`ZOrderGroups::take_verify`] が引き取る）。
    pub(crate) fn arm_verify(&mut self, verify: GroupVerify) {
        self.verify = Some(verify);
    }

    /// 検証待ちを引き取る（引き取った時点で預かりは消える＝一回限りの検証）。
    pub(crate) fn take_verify(&mut self) -> Option<GroupVerify> {
        self.verify.take()
    }

    /// 検証待ちが残っているか（起床旗の判定に使う）。
    pub(crate) fn has_verify(&self) -> bool {
        self.verify.is_some()
    }

    /// 当該グループの連続失敗を 1 つ数えて、数えた後の値を返す。
    ///
    /// 上限で頭打ちにするのは、`u8` を一周させて「連続失敗 0 回」に見せないためである。
    pub(crate) fn note_verify_failure(&mut self, id: u32) -> u8 {
        let streak = self.fail_streaks.entry(id).or_insert(0);
        *streak = streak.saturating_add(1);
        *streak
    }

    /// 当該グループの連続失敗を 0 へ戻す（成立した巡に呼ぶ）。
    pub(crate) fn clear_fail_streak(&mut self, id: u32) {
        self.fail_streaks.remove(&id);
    }

    /// 当該グループの連続失敗数（一度も失敗していなければ 0）。
    pub(crate) fn fail_streak(&self, id: u32) -> u8 {
        self.fail_streaks.get(&id).copied().unwrap_or(0)
    }

    /// そのグループを今も維持の対象にしているか（頭打ちに達していないか）。
    ///
    /// 判定を**グループ ID ごと**に閉じてあることが、要件 8.2 の「1 つの不成立が他の
    /// グループの是正を止めない」の実質である。全体で 1 つの数を持つと、あるグループの
    /// 環境側の失敗が無関係なグループの維持まで打ち切る。
    pub(crate) fn is_maintained(&self, id: u32) -> bool {
        self.fail_streak(id) < VERIFY_FAIL_CAP
    }

    /// すべてのグループの連続失敗を 0 へ戻す（維持がひと区切りついた巡に呼ぶ）。
    ///
    /// 呼ぶのは印（[`Self::pending`]）が降りる時点である。印が降りている間は維持系が
    /// 何もしないので、頭打ちで外したグループが実際に維持対象へ戻るのは**次の追随
    /// トリガが印を立て直した巡**になる——design の「外したグループは次の追随トリガで
    /// 維持対象へ戻す」を、トリガ側の書き方（`pending = true`）に一切依存せずに満たす
    /// 形である。
    pub(crate) fn clear_all_fail_streaks(&mut self) {
        self.fail_streaks.clear();
    }
}

/// 検証の連続失敗をどこで打ち切るか（design「頭打ち」の 3 回）。
///
/// 打ち切るのは**そのグループだけ**であり、他のグループの是正も、印の解除も止めない。
/// 打ち切りは記録（warn）を伴い、次の追随トリガで解ける（[`ZOrderGroups::is_maintained`]／
/// [`ZOrderGroups::clear_all_fail_streaks`]）。
pub(crate) const VERIFY_FAIL_CAP: u8 = 3;

// ============================================================================
// GroupProbe - 実測の口（判断を実機から切り離すための一線）
// ============================================================================

/// 観測が外界へ触れる唯一の口。
///
/// 「`Entity` から `HWND` を引く」のは World を持つ維持系にしかできず、「前面走査」は
/// Win32 にしかできない。その 2 つだけを trait の向こうへ追い出すことで、観測の**組立**
/// （順序の保存・未解決の数え方・相対順の判定）は実機も World も無しに固定できる
/// （要件 10.1）。既存ペア機構が観測値を構造体へ写して判断へ渡しているのと同じ分割で
/// あり、グループでは列の長さが可変ゆえ「値の写し」ではなく「引く口」の形にしてある。
pub(crate) trait GroupProbe {
    /// 窓の実体からハンドルを引く（まだ窓が無ければ `None`）。
    fn resolve(&self, entity: Entity) -> Option<HWND>;

    /// 指定窓より手前に居る**可視の**窓を、手前へ向かって集める。
    ///
    /// 既定実装が既存の実測層をそのまま呼ぶ——走査を書き直さないことをここで構造的に
    /// 決めている（実装側は [`GroupProbe::resolve`] だけを書けばよい）。差し替えるのは
    /// 決定論的テストだけである。
    fn scan_in_front(&self, hwnd: HWND) -> FrontScan {
        measure_windows_in_front(hwnd)
    }
}

// ============================================================================
// 観測——グループの内側の相対順だけを見る
// ============================================================================

/// 1 グループの観測結果。
///
/// **グループの外側については何も持たない**——他グループの窓がどこに居るか、構成外の窓が
/// 間に挟まっているか、といった値をそもそも記録しない。これが「異なるグループどうしの
/// 相対的な前後関係を固定の規則で決定しない」（要件 3.6）と「グループに属していない窓
/// どうしの相対順を変えない」（要件 6.1／6.2）の構造的な根拠である——判断
/// （[`decide_group_fix`]）はこの型しか見ないので、書きたくても書けない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GroupObservation {
    /// 対象グループ
    pub id: u32,
    /// 解決できたメンバーのみ（順序保存）
    pub hwnds: Vec<HWND>,
    /// 前面走査が**実際に出会った**構成窓の列（手前から順）
    pub measured_front: Vec<HWND>,
    /// ハンドル未解決の数（要件 8.4 の記録材料）
    pub missing: usize,
    /// 前面走査による相対順の成否
    pub order_ok: bool,
    /// 前面走査が最前面まで辿れたか（走査を行わなかった巡は `None`）
    ///
    /// [`measured_front`](Self::measured_front) は走査が出会わなかったメンバーを落とす
    /// ので、その欄だけでは「測ったら別の場所に居た」と「そこまで測れなかった」が同じ
    /// 字面になる（走査は上限 512 枚で打ち切られ得る）。打ち切りは走査層が warn に残す
    /// が、それは別の出力先の 2 行目であり、突き合わせて初めて解ける形は記録行の設計
    /// 理由（1 行で読める）が戒めているものそのものである。よって走査の完否をここで
    /// 運び、検証不一致の行へ同じ 1 行として載せる。
    ///
    /// 3 値なのは「測っていない」を `false`（＝測ったが最前面まで届かなかった）へ
    /// 潰さないためである（既存ペア機構の `tristate_field` と同じ規律）。
    pub scan_complete: Option<bool>,
}

/// 観測と判断を 1 対にした 1 グループ分の計画。
pub(crate) struct GroupPlan {
    /// そのグループを実測した結果
    pub observation: GroupObservation,
    /// その観測から出た 3 択
    pub decision: GroupFixDecision,
}

/// 各グループを**互いに独立に**観測して判断する（グループ間の順序は一切見ない）。
///
/// # 受け口が空なら何も起きない（要件 6.1／6.4）
///
/// 渡された列が空のとき、本関数は [`GroupProbe`] を一度も呼ばず、
/// [`decide_group_fix`] を一度も呼ばず、空の計画を返す。既定状態（グループ指定が
/// 一つも無い状態）で新系統が実測すらしないのはこの形による——「指令を出さない判断を
/// する」のではなく「判断の機会がそもそも作られない」。
///
/// # なぜ 1 本ずつ独立に回すのか
///
/// グループをまたいで並べ替える規則は正典に無く（要件 3.6）、作れば「作者が指定して
/// いない前後関係をエンジンが決めた」ことになる。ループの内側が 1 グループの
/// [`ZOrderGroupSpec`] しか見ないことで、その規則を書く場所を残さない。
pub(crate) fn plan_group_fixes<P: GroupProbe + ?Sized>(
    groups: &[ZOrderGroupSpec],
    probe: &P,
) -> Vec<GroupPlan> {
    groups
        .iter()
        .map(|spec| {
            let observation = observe_group(spec, probe);
            let decision = decide_group_fix(&observation);
            GroupPlan {
                observation,
                decision,
            }
        })
        .collect()
}

/// 1 グループを観測する（ハンドルの解決と、指定の相対順が成立しているかの実測）。
///
/// 解決できなかったメンバーは列から落ちるが**順序は詰めるだけ**であり、残ったメンバーの
/// 相対順は宣言どおりに保たれる（要件 1.4——存在する窓だけで指定順を成立させる）。
/// 落ちた数は [`GroupObservation::missing`] に残り、要件 8.4 の記録材料になる。
///
/// 走査は**列の末尾（最も奥に居るべき窓）から手前へ**行う。末尾より手前に、残るメンバーが
/// 宣言の逆順で現れれば相対順は成立している。1 本の走査で済むのは、相対順の判定に必要な
/// のが「誰が誰より手前か」だけだからである。
///
/// 解決できた窓が 2 枚未満のときは走査そのものを行わない——比べる相手が居らず、
/// Win32 を呼ぶ理由が無い。
pub(crate) fn observe_group<P: GroupProbe + ?Sized>(
    spec: &ZOrderGroupSpec,
    probe: &P,
) -> GroupObservation {
    let mut hwnds = Vec::with_capacity(spec.members.len());
    let mut missing = 0usize;
    for member in &spec.members {
        match probe.resolve(*member) {
            Some(hwnd) => hwnds.push(hwnd),
            None => missing += 1,
        }
    }

    let (order_ok, measured_front, scan_complete) = match hwnds.last() {
        Some(back) if hwnds.len() >= 2 => {
            let scan = probe.scan_in_front(*back);
            (
                order_holds(&hwnds, &scan),
                measured_members(&hwnds, &scan),
                Some(scan.reached_top),
            )
        }
        // 比べる相手が居ないので「崩れている」とは言えない（判断側が別の理由で見送る）。
        // 測っていない以上、実測の列は空のままにし、走査の完否も番兵にする——宣言列を
        // 写せば「測った」と読め、`false` を書けば「測ったが届かなかった」と読める。
        _ => (true, Vec::new(), None),
    };

    GroupObservation {
        id: spec.id,
        hwnds,
        measured_front,
        missing,
        order_ok,
        scan_complete,
    }
}

/// 前面走査が**実際に出会った**構成窓を、手前から順に並べ直す（Win32 も World も触らない）。
///
/// `hwnds` は手前から順の解決済みメンバー、`scan` はその**末尾**から手前へ辿った走査結果
/// （近い順）である。走査は奥から手前へ進むので、出会った順をそのまま並べると奥から順に
/// なる——逆順にしてから、走査の起点である末尾を最後尾に足すと、記録に載せられる
/// 「手前から順」の実測列になる。
///
/// # なぜ判定用の bool だけでは足りないのか（要件 9.1／9.2）
///
/// 相対順の成否（[`order_holds`]）は 1 ビットであり、「どう違っていたか」を持たない。
/// 記録に宣言列を載せると、**まったく別の重なりが同じ字面の行を出す**——解決できた
/// メンバー集合さえ同じなら行は byte 一致になる。それでは「どの窓がどの窓のすぐ手前へ
/// 着いたか」に答えられず、とりわけ検証不一致の行が、不一致を報せながら期待どおりの
/// 並びを見せることになる。よって走査の結果そのものを値として残す。
///
/// # 構成窓だけを残す（要件 3.6／6.1）
///
/// 走査には構成外の窓も入っているが、ここで濾し落とす。観測結果がグループの外側について
/// 何も持たないという [`GroupObservation`] の不変条件は、この列にも等しくかかる——
/// 他人の窓の位置を持てば、それを見て動かす規則を書く場所ができてしまう。
///
/// 走査に現れなかったメンバー（本当に奥に居るか、走査が打ち切られたか）は列に載らない。
/// 載せないのが正しい——載せれば「測れなかった」が「測って、そこに在った」に化ける。
pub(crate) fn measured_members(hwnds: &[HWND], scan: &FrontScan) -> Vec<HWND> {
    let Some(back) = hwnds.last().copied() else {
        return Vec::new();
    };
    let mut found: Vec<HWND> = scan
        .windows
        .iter()
        .copied()
        .filter(|seen| *seen != back && hwnds.contains(seen))
        .collect();
    found.reverse();
    found.push(back);
    found
}

/// 前面走査の結果から、宣言どおりの相対順が成立しているかを決める（Win32 も World も触らない）。
///
/// `hwnds` は手前から順の解決済みメンバーであり、`scan` はその**末尾**から手前へ辿った
/// 走査結果（近い順）である。末尾を除くメンバーが、宣言の**逆順**で `scan` の中に
/// 部分列として現れれば成立である。
///
/// # 部分列でよい理由（要件 3.6／6.1）
///
/// メンバーとメンバーの間に構成外の窓が何枚挟まっていても、判定は変わらない。
/// 「隣接していること」ではなく「相対順が守られていること」だけを見るのが正典の意味論で
/// あり、間の窓を詰めようとした瞬間、作者が指定していない窓の前後を動かすことになる。
///
/// # 測り切れなかったときは是正が要る側へ倒す
///
/// 走査が失敗ないし上限で打ち切られた場合（[`FrontScan::reached_top`] が偽）、メンバーを
/// 見つけられなかったのが「本当に奥に居る」からなのか「そこまで辿れなかった」からなのかは
/// 区別できない。ここでは**成立していない**側へ倒す——是正は宣言どおりの順へ並べ直すだけの
/// 冪等な操作であり、余分に出しても作者の指定から外れないのに対し、成立側へ倒すと崩れた
/// 重なりを黙って放置することになるからである。倒した事実そのものは、走査側
/// （[`measure_windows_in_front`]）が失敗・打切りを必ず warn に残している。
pub(crate) fn order_holds(hwnds: &[HWND], scan: &FrontScan) -> bool {
    // 末尾は走査の起点そのものなので、探すのはそれより手前に並ぶべき残りのメンバーである。
    let Some((_, front_side)) = hwnds.split_last() else {
        return true;
    };
    // 走査は手前へ向かって進むため、現れるべき順は宣言の逆順になる。
    let mut expected = front_side.iter().rev();
    let Some(mut want) = expected.next().copied() else {
        return true;
    };
    for seen in &scan.windows {
        if *seen == want {
            match expected.next() {
                Some(next) => want = *next,
                None => return true,
            }
        }
    }
    false
}

// ============================================================================
// decide_group_fix - 是正要否の純判断
// ============================================================================

/// 是正を見送った理由（要件 8.3——理由の無い見送りを型として作らせない）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GroupSkipReason {
    /// 既に宣言どおりの相対順である（収束の同値ガード＝指令 0 本）
    AlreadyOrdered,
    /// 解決できた窓が 2 枚未満で、並べ替えるべき相手が居ない
    TooFewResolved,
    /// 宣言されたメンバーの窓がまだ現れていない（要件 8.4——他の窓の配置は続ける）
    MemberMissing,
    /// この巡は既存のペア機構が是正を出しており、重ねて出さない（調停）
    PairFixThisPass,
    /// 検証の連続失敗が上限に達したので、このグループの維持を打ち切った（要件 8.2／8.3）
    ///
    /// 「諦めた」を語として持つのは、これが**記録の無い断念**と紙一重の判断だからである
    /// ——理由語のある見送りとして残る限り、読み手は「何度やっても環境側が受け付けな
    /// かった」という事実に到達できる。水準だけは他の見送りと違い warn である
    /// （[`record_group_give_up`]）。
    GaveUpAfterFailures,
}

/// 是正判断の結果。
///
/// 座標・寸法のフィールドを**一切持たない**——是正は表示順のみを動かす（要件 11.1）ことを、
/// 実行時の判定ではなく型の形で保証する。動かす窓も `Chain` の `chain` に載った窓に
/// 限られ、そこへ入るのは観測が集めた**構成窓だけ**である（要件 2.5）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GroupFixDecision {
    /// 何もしない（理由必須・要件 8.3）
    Skip(GroupSkipReason),
    /// `chain[i]` を直前要素（`i == 0` なら `head`）の直後へ差し込む
    Chain {
        /// 動かさない先頭の窓
        head: HWND,
        /// 手前から順に差し込む残りの窓
        chain: Vec<HWND>,
    },
}

/// グループの重なりをどう是正するかの純判断（Win32 も World も触らない）。
///
/// 判定は 3 択に閉じており、順は次のとおりである。
///
/// 1. **枚数**: 解決できた窓が 2 枚未満なら [`GroupSkipReason::TooFewResolved`]。
///    相対順という概念が立たないので、順序の成否より先に置く。
/// 2. **同値ガード**: 既に宣言どおりなら [`GroupSkipReason::AlreadyOrdered`]——
///    **指令 0 本**。是正が誘発する重なりの変化で再是正が走る往復を、ここで断つ。
/// 3. **連鎖**: それ以外は先頭を動かさない連鎖。
///
/// # 先頭を動かさないのはなぜか
///
/// 作者が宣言したのは**グループの内側の相対順**だけであり、グループ全体をデスクトップの
/// どこへ置くかは宣言していない（要件 3.6）。先頭を動かせば、グループの外の窓との前後が
/// 副作用で変わる——指定していない前後関係をエンジンが決めたことになる。先頭を軸に
/// 残りを後ろへ繋ぐ形なら、動くのは構成窓どうしの相対位置だけで済む。
pub(crate) fn decide_group_fix(obs: &GroupObservation) -> GroupFixDecision {
    if obs.hwnds.len() < 2 {
        return GroupFixDecision::Skip(GroupSkipReason::TooFewResolved);
    }
    if obs.order_ok {
        return GroupFixDecision::Skip(GroupSkipReason::AlreadyOrdered);
    }
    let (head, rest) = obs
        .hwnds
        .split_first()
        .expect("2 枚以上あることは直前で確かめた");
    GroupFixDecision::Chain {
        head: *head,
        chain: rest.to_vec(),
    }
}

// ============================================================================
// 記録——マクロを呼ぶのはこのモジュールだけ
// ============================================================================

/// 記録の水準。
///
/// `tracing` のマクロは水準ごとに別のマクロであり、[`emit`] を 1 か所に閉じるには
/// 水準を値で渡すほかない。
enum GroupRecordLevel {
    /// 診断専用（既定運転では無音・サインオフの `RUST_LOG` で点灯）
    Debug,
    /// 不正な指定の拒否（既定運転でも残す・`logging.md` の「無効なパラメーター」区分）
    Warn,
    /// 検証不一致（既定運転でも残す）
    Error,
}

/// 本モジュール唯一の記録の出口（`tracing` のマクロが現れるのはここだけ）。
///
/// 出力先は module path 既定＝`wintf::ecs::window::zorder_group` であり、実機サインオフの
/// grep はこの名前で行う。他ファイルへマクロを置くと grep 対象が分裂して手順が静かに嘘に
/// なるため、記録を増やすときは**この下に `record_*`／`log_*` を足す**こと。
fn emit(level: GroupRecordLevel, line: &str) {
    match level {
        GroupRecordLevel::Debug => debug!("{line}"),
        GroupRecordLevel::Warn => warn!("{line}"),
        GroupRecordLevel::Error => error!("{line}"),
    }
}

/// 見送りを記録する（理由必須）。
///
/// `group_id` が `None` なのは巡そのものの見送り（既存ペア機構との調停）であり、
/// `observed` が `None` なのは観測より前に見送った場合である——どちらもフィールドは
/// 落とさず番兵にする。落とすと「記録が出ていない」と「その経路にはその値が無い」の
/// 区別が事後に付かなくなる。
pub(crate) fn record_group_skip(
    group_id: Option<u32>,
    reason: GroupSkipReason,
    observed: Option<&GroupObservation>,
) {
    emit(
        GroupRecordLevel::Debug,
        &skip_line(group_id, reason, observed),
    );
}

/// 判断結果を記録し、**適用すべき是正だけ**を返す（要件 8.3）。
///
/// 見送りはこの関数の中で必ず理由つきの記録になり、返り値としては何も返らない。
/// あわせて、まだ現れていないメンバーが居れば
/// [`GroupSkipReason::MemberMissing`] を記録する——こちらは**維持を止めない**
/// （要件 8.4——対応する窓が無いことを記録しつつ、他の窓の配置は継続する）。
///
/// 既存ペア機構の `record_decision` と同じく、これは**規約であって構造保証ではない**
/// ——[`decide_group_fix`] の結果を本関数へ通さずに握り潰す書き方は言語上は可能である。
/// 型で塞がないのは design.md が判断関数の署名を固定しているためであり、逸脱は
/// テストとレビューで検出する。
pub(crate) fn record_group_decision(
    obs: &GroupObservation,
    decision: GroupFixDecision,
) -> Option<GroupFixDecision> {
    if obs.missing > 0 {
        record_group_skip(Some(obs.id), GroupSkipReason::MemberMissing, Some(obs));
    }
    match decision {
        GroupFixDecision::Skip(reason) => {
            record_group_skip(Some(obs.id), reason, Some(obs));
            None
        }
        chain => Some(chain),
    }
}

/// 検証の結末（要件 8.3——「測れなかった」を成否のどちらかへ潰さない）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GroupVerifyOutcome {
    /// 実測が指令どおりだった（`fix` を記録済み）
    Matched,
    /// 実測が指令どおりでなかった（`verify-failed` を記録済み）
    Mismatched,
    /// 比べる相手が 2 枚に満たず、そもそも実測していない（見送りを記録済み）
    NotMeasured,
}

/// 適用後の実測照合を記録し、結末を返す（要件 9.1／9.2）。
///
/// - 一致: 是正の記録（debug）——出した指令と実測を同じ行に載せる。
/// - 不一致: 検証不一致の記録（error）。
/// - 実測なし: 理由つきの見送り（debug）。**是正の記録は出さない。**
///
/// 是正の記録をここでしか出さないのは、発行と同じ巡の実測が必ず書込前の値になり証跡に
/// 使えないからである（[`GroupVerify`] の doc 参照）。
///
/// # 測っていない巡を「成立」と読ませない
///
/// 検証の巡までに窓が減ると、解決できるのが 1 枚以下になり前面走査そのものが行われない
/// （[`observe_group`]）。このとき [`GroupObservation::order_ok`] は「崩れているとは
/// 言えない」の意味で真になるが、その真をここで成立として扱うと、**何も測っていない巡に
/// `fix` 行が出る**。サインオフはその行を「指定が成立した」と読むので、これは
/// 「証跡のふりをした非証跡」そのものである。よって判定材料は `order_ok` ではなく
/// [`GroupObservation::scan_complete`] の有無を先に見る——走査を行った巡だけが証跡に
/// なる。呼び出し側の作法ではなくこの関数の中で塞いであるのは、記録の唯一の入口を
/// 通る限り誰が呼んでも同じ保証が効くようにするためである。
pub(crate) fn record_group_verification(
    verify: &GroupVerify,
    observed: &GroupObservation,
) -> GroupVerifyOutcome {
    if observed.scan_complete.is_none() {
        // 実測していない以上、成否のどちらも主張できない。黙って落とすと要件 8.3 に
        // 触れるので、既存の理由語（比べる相手が居ない）で見送りとして残す。
        record_group_skip(
            Some(verify.id),
            GroupSkipReason::TooFewResolved,
            Some(observed),
        );
        return GroupVerifyOutcome::NotMeasured;
    }
    if observed.order_ok {
        emit(GroupRecordLevel::Debug, &fix_line(verify, observed));
        GroupVerifyOutcome::Matched
    } else {
        emit(
            GroupRecordLevel::Error,
            &verify_failed_line(verify, observed),
        );
        GroupVerifyOutcome::Mismatched
    }
}

/// そのグループの維持を打ち切った事実を記録する（warn・要件 8.2／8.3）。
///
/// 水準が warn なのは、諦めは診断手順を有効化していない通常運転でも読めなければ
/// 「黙って諦めた」に等しいからである（見送り＝debug との差はここにある）。ゴーストは
/// 異常終了させない——記録して次の追随トリガを待つのが要件 8.2 の求める形である。
///
/// # 欄は既存の見送り行に足す形で増やす
///
/// 本文は [`skip_line`] が組む 5 欄（グループ・理由・解決枚数・未解決数・相対順の成否）に、
/// この経路でしか意味を持たない 2 欄を継ぎ足したものである（[`log_group_member_missing`]
/// と同じ作法——`skip_line` の書式そのものは動かさない）。
///
/// - `streak`: 連続して何回の検証に失敗した末の打ち切りか
/// - `scan_complete`: 最後の実測で走査が最前面まで辿れたか（打ち切りの原因が「窓が
///   動かせない」のか「そもそも測り切れていない」のかを 1 行で切り分ける）
pub(crate) fn record_group_give_up(group_id: u32, streak: u8, observed: &GroupObservation) {
    let line = format!(
        "{base} streak={streak} scan_complete={scan}",
        base = skip_line(
            Some(group_id),
            GroupSkipReason::GaveUpAfterFailures,
            Some(observed)
        ),
        scan = tristate_field(observed.scan_complete),
    );
    emit(GroupRecordLevel::Warn, &line);
}

// 受理（`[zorder-group] applied`）と拒否（`[zorder-group] rejected`）の記録は、
// 要件 9.5 の保全対象として本モジュールの退役後も残る。よって定数・行組立・記録関数の
// いずれも `zorder_chain_diag` へ**字面を 1 字も変えずに**移してある。呼び出し元は
// `wintf::ecs::window` の再輸出を通して同じ名前で呼び続ける。

/// 宣言されたメンバーのうち**まだ現れていない窓**があった事実を記録する（要件 8.4）。
///
/// 呼ぶのは台帳から窓の列を組む層＝areka の射影の相である。射影は「実在する窓だけ」を
/// 抜き出すので、まだ生まれていない窓・破棄済みの窓はこの Resource へ届く前に落ちる
/// ——落ちたことをここで報せなければ、記録の上では**最初から書かれていなかった**のと
/// 区別が付かない（要件 8.3 が禁じる「黙って諦める」の一形態である）。
///
/// 理由語は新設せず既存の [`GroupSkipReason::MemberMissing`] を用いる——記録を読む側の
/// 語彙を増やさないためである（同じ事実は同じ語で出る）。
///
/// # 観測側の 3 欄は番兵のまま・射影側の実数は別名で足す
///
/// [`record_group_skip`] が組む `resolved`／`missing`／`order_ok` は**前面走査の結果**を
/// 載せる欄であり、ここはその走査より手前なので番兵（`-`）になる。呼び手が知っている
/// 実数をその 3 欄へ流し込むことはしない——同じ欄名が場所によって別の出所を指すのは、
/// 「実測を騙る宣言列」を載せていた `fix` 行の欠陥と同型だからである。
///
/// 代わりに、射影の段でしか分からない 2 つの実数を**衝突しない名前**で末尾へ足す。
///
/// - `declared`: 作者が書いた要素の数（台帳に載っている宣言の長さ）
/// - `existing`: そのうち実在する窓へ解決できた数
///
/// 引き算した「欠けた数」ではなく両方の生の数を載せるのは、`existing=0`（一度も現れて
/// いない）と `existing=2`（一部だけ現れた）が要件 8.4 の読み手にとって別の事実だから
/// である。行の組み立てがこちらに在るのは、`skip` 行そのものの書式（[`skip_line`]）を
/// 動かさずに欄を足すためである。
///
/// [`skip_line`]: super::zorder_group_diag::skip_line
pub fn log_group_member_missing(group_id: u32, declared: usize, existing: usize) {
    let line = format!(
        "{base} declared={declared} existing={existing}",
        base = skip_line(Some(group_id), GroupSkipReason::MemberMissing, None),
    );
    emit(GroupRecordLevel::Debug, &line);
}

#[cfg(test)]
#[path = "zorder_group_decision_tests.rs"]
mod zorder_group_decision_tests;
