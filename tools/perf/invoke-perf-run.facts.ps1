#Requires -Version 7.0
<#
================================================================================
invoke-perf-run.facts.ps1 — ランナーが run-meta.txt に書くマシンとソースの識別
  spec: audit-tools-perf-run（ランナーを 1,000 行の下に保つため invoke-perf-run.ps1 から移した）

読み込み用。invoke-perf-run.ps1 が perf-tool-base.ps1 の後で
    . (Join-Path $PSScriptRoot 'invoke-perf-run.facts.ps1')
として読み込む。単独で回しても何もしない。頼ってよいのは perf-tool-base.ps1 の関数だけ。
================================================================================
#>

# マシン識別（要件 4.5 の「測定マシンの条件」素材）。取得できない項目は理由を値として残す。
function Get-MachineFacts {
    $facts = [ordered]@{}
    $facts['hostname']              = [Environment]::MachineName
    $facts['logical_processors']    = [Environment]::ProcessorCount
    $facts['os_version']            = [Environment]::OSVersion.VersionString
    $facts['powershell_version']    = $PSVersionTable.PSVersion.ToString()
    $facts['dotnet_version']        = [Environment]::Version.ToString()

    try {
        $os = Get-CimInstance -ClassName Win32_OperatingSystem -ErrorAction Stop
        $facts['os_caption']        = $os.Caption
        $facts['os_build']          = $os.BuildNumber
        $facts['total_memory_mb']   = [math]::Round($os.TotalVisibleMemorySize / 1024)
    } catch {
        $facts['os_caption']        = "(unavailable: $($_.Exception.Message))"
    }
    try {
        $cpu = @(Get-CimInstance -ClassName Win32_Processor -ErrorAction Stop)
        $facts['cpu_model']         = ($cpu | ForEach-Object { $_.Name.Trim() }) -join ' / '
        $facts['cpu_sockets']       = $cpu.Count
        $facts['cpu_cores_physical']= ($cpu | Measure-Object -Property NumberOfCores -Sum).Sum
        $facts['cpu_max_clock_mhz'] = ($cpu | Measure-Object -Property MaxClockSpeed -Maximum).Maximum
    } catch {
        $facts['cpu_model']         = "(unavailable: $($_.Exception.Message))"
    }
    $facts['power_scheme']          = Get-ActivePowerScheme
    return $facts
}

# 今の電源設定を powrprof の API から '<GUID> (<名前>)' の形で取る。名前は UTF-16 で来るので
# 端末の文字コードに依らない（powercfg は端末の文字コードで書き、437 の端末では日本語の名前が ? に落ちる）。
# 取れないときは '(unavailable: <理由>)' を返す（走行は止めない）。
function Get-ActivePowerScheme {
    try {
        if (-not ('ArekaPerfRun.PowerScheme' -as [type])) {
            Add-Type -Namespace ArekaPerfRun -Name PowerScheme -MemberDefinition @'
[DllImport("powrprof.dll")]
public static extern uint PowerGetActiveScheme(IntPtr userRootPowerKey, out IntPtr activePolicyGuid);
[DllImport("powrprof.dll")]
public static extern uint PowerReadFriendlyName(IntPtr rootPowerKey, ref Guid schemeGuid, IntPtr subGroupGuid, IntPtr settingGuid, byte[] buffer, ref uint bufferSize);
[DllImport("kernel32.dll")]
public static extern IntPtr LocalFree(IntPtr mem);
'@
        }
        $api = [ArekaPerfRun.PowerScheme]
        $guidPtr = [IntPtr]::Zero
        $rc = $api::PowerGetActiveScheme([IntPtr]::Zero, [ref]$guidPtr)
        if ($rc -ne 0) { return "(unavailable: PowerGetActiveScheme returned $rc)" }
        try {
            $guid = [System.Runtime.InteropServices.Marshal]::PtrToStructure($guidPtr, [type][guid])
        } finally {
            [void]$api::LocalFree($guidPtr)
        }
        # 1 回目で大きさを聞き、2 回目で名前（UTF-16・終端の NUL 付き）を読む
        $size = [uint32]0
        $rc = $api::PowerReadFriendlyName([IntPtr]::Zero, [ref]$guid, [IntPtr]::Zero, [IntPtr]::Zero, $null, [ref]$size)
        if ($rc -ne 0 -or $size -eq 0) { return "$guid (unavailable: PowerReadFriendlyName returned $rc)" }
        $buffer = [byte[]]::new($size)
        $rc = $api::PowerReadFriendlyName([IntPtr]::Zero, [ref]$guid, [IntPtr]::Zero, [IntPtr]::Zero, $buffer, [ref]$size)
        if ($rc -ne 0) { return "$guid (unavailable: PowerReadFriendlyName returned $rc)" }
        $name = [System.Text.Encoding]::Unicode.GetString($buffer, 0, [int]$size).TrimEnd([char]0)
        return "$guid ($name)"
    } catch {
        return "(unavailable: $($_.Exception.Message))"
    }
}

# ソース識別（同一手順の再測が同一コードに対して行われたかを後から突き合わせるため）
function Get-SourceFacts {
    param([string]$RepoRoot)
    $facts = [ordered]@{}
    try {
        # git は UTF-8 で書くので、共通部品の Invoke-Child（既定で UTF-8）で読む
        $git = Get-Command git -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
        if (-not $git) { throw 'command not found: git' }
        $head = Invoke-Child -Exe $git.Source -Arguments @('-C', $RepoRoot, 'rev-parse', 'HEAD') -Quiet
        if ($head.Code -eq 0) {
            $facts['git_head'] = ($head.Out -join "`n").Trim()
            $branch = Invoke-Child -Exe $git.Source -Arguments @('-C', $RepoRoot, 'rev-parse', '--abbrev-ref', 'HEAD') -Quiet
            if ($branch.Code -eq 0) { $facts['git_branch'] = ($branch.Out -join "`n").Trim() }
            $status = Invoke-Child -Exe $git.Source -Arguments @('-C', $RepoRoot, 'status', '--porcelain') -Quiet
            if ($status.Code -eq 0) {
                $facts['git_dirty_files'] = @($status.Out | Where-Object { $_.Trim().Length -gt 0 }).Count
            }
        } else {
            $facts['git_head'] = '(unavailable: git rev-parse HEAD failed)'
        }
    } catch {
        $facts['git_head'] = "(unavailable: cannot run git: $($_.Exception.Message))"
    }
    return $facts
}
