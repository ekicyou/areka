//! 広がりの予算（指摘 3・21・22）: 入れ子をたどる走査が 1 回の範囲でたどってよい仕事の量を数え、
//! 尽きたら入るのを断る。菱形の参照（1 つの面が同じ面を何度も insert する）で広がりが
//! 指数になっても、走査が固まらないようにする。深さではなく「入る面と積み荷の数」を数える
//! （積み荷＝命令列の段の命令と子へ入る印・当たり判定の持ち込みで親へ渡す領域。外形とシェルの外形は入る面だけ）。

use crate::world::Records;

/// 1 回の走査の範囲で数えてよい「入る面と積み荷」の数。
/// 根拠（3.2・2026-11-06）: 面の数だけを数えていたときの測り。release・NAGI（Intel Core Ultra 9 185H）・15 段の菱形で 1 面あたり約 226 ns＝16.667 ms に約 73,000 面→2^16 に切り下げ。
/// 6.1 で積み荷も数える単位に変えた（同じ菱形で 1 面あたり約 3 つ＝1 つあたり約 75 ns の見積もり）ので、測り直しの候補（見積もりは 2^17）。
pub(crate) const VISIT_BUDGET: usize = 1 << 16;

/// 走査 1 回の範囲の残りの数と、報告済みの印。
#[derive(Debug)]
pub(crate) struct Budget {
    limit: usize,
    left: usize,
    reported: bool,
}

impl Budget {
    /// 記録を出す予算。本番の口は [`VISIT_BUDGET`] を、テストは小さな数を渡す。
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

    /// 面に 1 つ入る前に呼ぶ。残りがあれば減らして true。尽きていれば、範囲で初めての
    /// ときだけ `error!`（`walk`＝走査の名前・`root`＝根の面・予算）を出して false。
    pub(crate) fn enter(&mut self, walk: &'static str, root: u32) -> bool {
        self.take_up_to(1, walk, root) == 1
    }

    /// 積み荷 `n` 個を積む前に呼ぶ（指摘 21・22）。残りの内の数（`n` 以下）を引いて返し、足りなければ
    /// [`Budget::enter`] と同じく範囲で初めてのときだけ `error!` を出す。呼び手は返った数だけ積む。
    pub(crate) fn take_up_to(&mut self, n: usize, walk: &'static str, root: u32) -> usize {
        let granted = n.min(self.left);
        self.left -= granted;
        if granted < n && !self.reported {
            self.reported = true;
            tracing::error!(
                target: "areka_emo_compose",
                walk,
                root,
                budget = self.limit,
                "入れ子の広がりが予算を越えた: まだ入っていない枝を捨てる"
            );
        }
        granted
    }

    /// 尽きたことを記録したか（黙らせる組み方では初めから true）。
    pub(crate) fn reported(&self) -> bool {
        self.reported
    }

    /// 覚えた子の分 `n`（子に入る 1 と子の下で数えた数）をまとめて引く。足りなければ何も引かず、
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
                entered.push(budget.enter("plan", 7));
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

    /// 黙らせる組み方では、同じく断るが記録は出さない。
    #[test]
    fn quiet_budget_refuses_without_records() {
        let mut entered = Vec::new();
        let out = capture_logs(|| {
            let mut budget = Budget::with_records(1, Records::Quiet);
            entered.push(budget.enter("hit_import", 0));
            entered.push(budget.enter("hit_import", 0));
        });
        assert_eq!(entered, [true, false]);
        assert!(out.is_empty(), "記録は出さない: {out:?}");
    }
}
