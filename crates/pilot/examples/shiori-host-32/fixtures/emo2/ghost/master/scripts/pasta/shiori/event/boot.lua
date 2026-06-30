---@module pasta.shiori.event.boot
--- OnBoot 時間帯ディスパッチャ
---
--- 起動時刻に応じて4バンドのDSLシーンにディスパッチする。
--- 各バンドには複数の同名シーンが存在し、pasta.dllがランダム選択する。
---   朝 (5-10時)  → 起動朝
---   昼 (11-17時) → 起動昼
---   夜 (18-23時) → 起動夜
---   深夜 (0-4時) → 起動深夜

local REG = require "pasta.shiori.event.register"
local SCENE = require "pasta.scene"

--- 時刻から起動バンドシーン名を返す
--- @param hour integer 0-23
--- @return string バンドシーン名
local function get_boot_band(hour)
    if hour >= 5 and hour <= 10 then
        return "起動朝"
    elseif hour >= 11 and hour <= 17 then
        return "起動昼"
    elseif hour >= 18 and hour <= 23 then
        return "起動夜"
    else
        return "起動深夜"
    end
end

--- OnBoot ハンドラ: 時間帯に応じたDSLバンドシーンにディスパッチ
--- @param act ShioriAct actオブジェクト
--- @return thread|nil シーンコルーチン、またはnil
REG.OnBoot = function(act)
    local band = get_boot_band(act.req.date.hour)
    return SCENE.co_exec(act, band, nil, nil)
end

return REG
