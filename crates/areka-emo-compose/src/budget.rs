//! 広がりの予算（指摘 3・21・22・129・130）: 入れ子をたどる走査が 1 回の範囲でたどってよい仕事の量を数え、
//! 尽きたら入るのを断る。菱形の参照（1 つの面が同じ面を何度も insert する）で広がりが
//! 指数になっても、走査が固まらないようにする。深さではなく「入る面となめる行と積み荷の数」を数える。
//! どの走査も面に入るときに「入る 1 ＋その面の行の全部（[`surface_rows`]）」を引き、足りなければ入らない
//! （面 1 つに入ってする仕事はその面の行の数に比例するので、走査全体の仕事は予算×定数で抑えられる）。
//! 積み荷（命令列の段の命令と子へ入る印・当たり判定の持ち込みで親へ渡す領域）もこれとは別に数える。

use crate::normalized::SurfaceMaster;
use crate::world::Records;

/// 1 回の走査の範囲で数えてよい「入る面とその面の行の全部と積み荷」の数。
/// 根拠（3.2・2026-11-06）: 面の数だけを数えていたときの測り。release・NAGI（Intel Core Ultra 9 185H）・15 段の菱形で 1 面あたり約 226 ns＝16.667 ms に約 73,000 面→2^16 に切り下げ。
/// 6.1 で積み荷も、7.1 でなめる行も、8.1 で面の行の全部（[`surface_rows`]）も数える単位に変えたので、測り直しの候補
/// （`examples/compose_timing.rs` の `diamond13` が 1 単位あたりの時間を出す）。
pub(crate) const VISIT_BUDGET: usize = 1 << 16;

/// 面 1 つの行の全部の数（element・collision と、animation ごとに interval の 1 行＋pattern の数・飽和）。
/// 面に入った走査がなめうる行の上限なので、どの走査も入るときにこれで予算から引く（指摘 129・130）。
pub(crate) fn surface_rows(master: &SurfaceMaster) -> usize {
    let lines = master
        .elements
        .len()
        .saturating_add(master.collisions.len());
    master.animations.iter().fold(lines, |n, a| {
        n.saturating_add(1).saturating_add(a.patterns.len())
    })
}

/// 走査 1 回の範囲の残りの数と、報告済みの印。
#[derive(Debug)]
pub(crate) struct Budget {
    limit: usize,
    left: usize,
    reported: bool,
}

impl Budget {
    /// 記録を出す予算（テストの口・本番の走査は記録の出し分けを渡す [`Budget::with_records`]）。
    #[cfg(test)]
    pub(crate) fn new(limit: usize) -> Self {
        Self::with_records(limit, Records::Emit)
    }

    /// 記録の出し分けがある走査の口。`Records::Quiet` なら尽きても `error!` を出さない。
    pub(crate) fn with_records(limit: usize, records: Records) -> Self {
        Self {
            limit,
            left: limit,
            reported: !records.emits(),
        }
    }

    /// 面に 1 つ入る前に呼ぶ。入る 1 とその面の行の数 `rows`（[`surface_rows`]）をまとめて引けたら true。
    /// 足りなければ残りを使い切り、範囲で初めてのときだけ `error!`（`walk`＝走査の名前・`root`＝根の面・
    /// 予算）を出して false（呼び手は入らない）。どの予算にも収まらない面（入る分が予算そのものより多い）は
    /// 残りを使わずにその面だけ断る（後ろの兄弟は入れる＝見える部品の表がその子だけ外すのと揃う）。
    pub(crate) fn enter(&mut self, rows: usize, walk: &'static str, root: u32) -> bool {
        let n = rows.saturating_add(1);
        if n > self.limit {
            self.report(walk, root);
            return false;
        }
        self.take_up_to(n, walk, root) == n
    }

    /// 積み荷 `n` 個を積む前に呼ぶ（指摘 21・22）。残りの内の数（`n` 以下）を引いて返し、足りなければ
    /// [`Budget::enter`] と同じく範囲で初めてのときだけ `error!` を出す。呼び手は返った数だけ積む。
    pub(crate) fn take_up_to(&mut self, n: usize, walk: &'static str, root: u32) -> usize {
        let granted = n.min(self.left);
        self.left -= granted;
        if granted < n {
            self.report(walk, root);
        }
        granted
    }

    /// 範囲で初めて断ったときだけ `error!` を出す。
    fn report(&mut self, walk: &'static str, root: u32) {
        if self.reported {
            return;
        }
        self.reported = true;
        tracing::error!(
            target: "areka_emo_compose",
            walk,
            root,
            budget = self.limit,
            "入れ子の広がりが予算を越えた: まだ入っていない枝を捨てる"
        );
    }

    /// 尽きたことを記録したか（黙らせる組み方では初めから true）。
    pub(crate) fn reported(&self) -> bool {
        self.reported
    }

    /// 覚えた子の分 `n`（子に入る 1 と子の行と子の下で数えた数）をまとめて引く。足りなければ何も引かず、
    /// 記録も出さずに false（呼び手は今どおり 1 つずつ入り、尽きた所で [`Budget::enter`] が記録を出す）。
    pub(crate) fn take(&mut self, n: usize) -> bool {
        if self.left < n {
            return false;
        }
        self.left -= n;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::Budget;
    use crate::log_capture::capture_logs;
    use crate::world::Records;

    /// 予算 3 は 3 つ目まで入り、4 つ目から断る。`error!` は範囲で 1 行だけ。
    #[test]
    fn budget_of_three_enters_three_then_refuses_with_one_error() {
        let mut entered = Vec::new();
        let out = capture_logs(|| {
            let mut budget = Budget::new(3);
            for _ in 0..5 {
                entered.push(budget.enter(0, "plan", 7));
            }
        });
        assert_eq!(entered, [true, true, true, false, false]);
        assert_eq!(
            out.matches("level=ERROR").count(),
            1,
            "error! は 1 行: {out}"
        );
        assert!(out.contains("target=areka_emo_compose"), "{out}");
        assert!(out.contains("walk=\"plan\""), "走査の名前: {out}");
        assert!(out.contains("root=7"), "根の面: {out}");
        assert!(out.contains("budget=3"), "予算の値: {out}");
    }

    /// どの予算にも収まらない面は、残りを使わずにその面だけ断る（後ろの小さな面は入れる）。記録は 1 行。
    #[test]
    fn surface_larger_than_the_budget_is_refused_without_spending_the_rest() {
        let mut entered = Vec::new();
        let out = capture_logs(|| {
            let mut budget = Budget::new(4);
            entered.push(budget.enter(4, "plan", 0));
            entered.push(budget.enter(1, "plan", 0));
            entered.push(budget.enter(1, "plan", 0));
        });
        assert_eq!(entered, [false, true, true]);
        assert_eq!(out.matches("level=ERROR").count(), 1, "{out}");
    }

    /// 黙らせる組み方では、同じく断るが記録は出さない。
    #[test]
    fn quiet_budget_refuses_without_records() {
        let mut entered = Vec::new();
        let out = capture_logs(|| {
            let mut budget = Budget::with_records(1, Records::Quiet);
            entered.push(budget.enter(0, "hit_import", 0));
            entered.push(budget.enter(0, "hit_import", 0));
        });
        assert_eq!(entered, [true, false]);
        assert!(out.is_empty(), "記録は出さない: {out:?}");
    }
}
