<#
.SYNOPSIS
  Mouse input (SendInput) and screen capture at physical-pixel coordinates.

.DESCRIPTION
  Actions:
    move        move the cursor to (X, Y)
    click       move to (X, Y), left press, short pause, left release
    rightclick  move to (X, Y), right press, short pause, right release
    drag        left press at (X, Y), move to (X2, Y2) in -Steps steps (default 20), release
    shot        copy the rectangle (X, Y)-(X2, Y2) of the screen to the PNG file -Out
    cursor      move to (X, Y), then read the cursor shape (GetCursorInfo) and name it by
                comparing the handle with the 14 stock cursors (LoadCursorW(NULL, IDC_*));
                any other handle is 'custom'
                with -Press: left press at (X, Y), read and print the line with ' pressed=1',
                then left release and read again (the release is sent even if the read fails)
  Coordinates are physical pixels of the virtual screen. Every call first makes the process
  per-monitor DPI aware (V2), so 200% and 150% monitors do not shift the coordinates.
  After each move the cursor position is read back (GetCursorPos); a mismatch is a failure
  (input is refused on a locked desktop or under a screen saver).
  A press is always followed by its release, even on an exception.
  The tool keeps no state and does not know about permission: callers send input only while
  they hold the exclusive-test desk and the developer allowed SendInput.

  Output (ASCII, one line):
    exit 0  ok <action> x=<X> y=<Y>
            ok cursor x=<X> y=<Y> shape=<ARROW|IBEAM|...|HELP|custom> handle=0x<hex>
            (with -Press: the same line with ' pressed=1' first, then the released line)
    exit 1  error <reason>
    exit 2  error <reason>   (missing or malformed arguments)

  Run: pwsh -NoProfile -File tools/mouse-input.ps1 -Action <action> -X <int> -Y <int>
         [-X2 <int> -Y2 <int>] [-Steps <int>] [-Out <absolute PNG path>] [-Press]
#>
#Requires -Version 7
param(
    [string]$Action,
    [string]$X,
    [string]$Y,
    [string]$X2,
    [string]$Y2,
    [string]$Steps = '20',
    [string]$Out,
    [switch]$Press
)

Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'

function Exit-With([int]$Code, [string]$Line) {
    # Exception messages can be localized; the terminal gets ASCII only.
    Write-Output ($Line -replace '[^\x20-\x7E]', '?')
    exit $Code
}

function ConvertTo-Int([string]$Name, [string]$Text) {
    $value = 0
    if ([string]::IsNullOrEmpty($Text) -or -not [int]::TryParse($Text, [ref]$value)) {
        Exit-With 2 "error -$Name needs an integer"
    }
    return $value
}

# ---- arguments ---------------------------------------------------------------
$actions = @('move', 'click', 'rightclick', 'drag', 'shot', 'cursor')
if ($actions -notcontains $Action) {
    Exit-With 2 "error -Action must be one of: $($actions -join ', ')"
}
if ($Press -and $Action -ne 'cursor') { Exit-With 2 'error -Press works only with -Action cursor' }
$px = ConvertTo-Int 'X' $X
$py = ConvertTo-Int 'Y' $Y
$needEnd = $Action -in @('drag', 'shot')
if ($needEnd) {
    $px2 = ConvertTo-Int 'X2' $X2
    $py2 = ConvertTo-Int 'Y2' $Y2
}
$stepCount = ConvertTo-Int 'Steps' $Steps
if ($stepCount -lt 1) { Exit-With 2 'error -Steps must be 1 or more' }
if ($Action -eq 'shot') {
    if ($px2 -le $px -or $py2 -le $py) { Exit-With 2 'error shot needs X2 > X and Y2 > Y' }
    if ([string]::IsNullOrEmpty($Out) -or -not [System.IO.Path]::IsPathFullyQualified($Out)) {
        Exit-With 2 'error -Out needs an absolute path'
    }
    if (-not (Test-Path -LiteralPath (Split-Path -Parent $Out) -PathType Container)) {
        Exit-With 2 'error the parent folder of -Out does not exist'
    }
}

# ---- Win32 -------------------------------------------------------------------
# INPUT / MOUSEINPUT are copied from tools/perf/invoke-followup-checks.ps1 (passed on the real machine).
if (-not ('AkMouseInputW32' -as [type])) {
    Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;

public class AkMouseInputW32 {
    [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr value);
    [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT pt);
    [DllImport("user32.dll", SetLastError=true)] public static extern uint SendInput(uint count, [In] INPUT[] inputs, int size);
    [DllImport("user32.dll")] public static extern int GetSystemMetrics(int index);
    [DllImport("user32.dll", SetLastError=true)] public static extern bool GetCursorInfo(ref CURSORINFO info);
    [DllImport("user32.dll")] public static extern IntPtr LoadCursorW(IntPtr instance, IntPtr id);

    [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
    [StructLayout(LayoutKind.Sequential)] public struct MOUSEINPUT {
        public int dx, dy; public uint mouseData, dwFlags, time; public IntPtr dwExtraInfo;
    }
    // On x64, INPUT is 40 bytes (type 4 + padding 4 + MOUSEINPUT 32). Adding the padding by hand
    // makes it 48 bytes and SendInput returns ERROR_INVALID_PARAMETER(87). Do not add it.
    [StructLayout(LayoutKind.Sequential)] public struct INPUT { public uint type; public MOUSEINPUT mi; }
    [StructLayout(LayoutKind.Sequential)] public struct CURSORINFO {
        public int cbSize; public uint flags; public IntPtr hCursor; public POINT ptScreenPos;
    }

    public static int InputSize() { return Marshal.SizeOf(typeof(INPUT)); }
}
"@
}

# DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2 = -4 (all coordinates below are physical pixels)
$null = [AkMouseInputW32]::SetProcessDpiAwarenessContext([IntPtr](-4))

$MOVE = 0x0001; $LEFTDOWN = 0x0002; $LEFTUP = 0x0004; $RIGHTDOWN = 0x0008; $RIGHTUP = 0x0010
$ABSOLUTE = 0x8000; $VIRTUALDESK = 0x4000
$vx = [AkMouseInputW32]::GetSystemMetrics(76)  # SM_XVIRTUALSCREEN
$vy = [AkMouseInputW32]::GetSystemMetrics(77)  # SM_YVIRTUALSCREEN
$vw = [AkMouseInputW32]::GetSystemMetrics(78)  # SM_CXVIRTUALSCREEN
$vh = [AkMouseInputW32]::GetSystemMetrics(79)  # SM_CYVIRTUALSCREEN

function Send-Mouse([int]$AtX, [int]$AtY, [uint32]$Flags) {
    $item = New-Object AkMouseInputW32+INPUT
    $item.type = 0
    $mouse = New-Object AkMouseInputW32+MOUSEINPUT
    # Windows maps normalized n back to pixel (n * size) >> 16, so ceil(p * 65536 / size) lands on p exactly.
    $mouse.dx = [int][math]::Floor((([long]($AtX - $vx)) * 65536 + $vw - 1) / $vw)
    $mouse.dy = [int][math]::Floor((([long]($AtY - $vy)) * 65536 + $vh - 1) / $vh)
    $mouse.dwFlags = $Flags -bor $MOVE -bor $ABSOLUTE -bor $VIRTUALDESK
    $item.mi = $mouse
    $sent = [AkMouseInputW32]::SendInput(1, @($item), [AkMouseInputW32]::InputSize())
    if ($sent -ne 1) {
        $err = [System.Runtime.InteropServices.Marshal]::GetLastWin32Error()
        throw "SendInput refused (lasterr=$err; locked desktop or screen saver?)"
    }
}

function Assert-CursorAt([int]$AtX, [int]$AtY) {
    Start-Sleep -Milliseconds 30
    $p = New-Object AkMouseInputW32+POINT
    $null = [AkMouseInputW32]::GetCursorPos([ref]$p)
    if ($p.X -ne $AtX -or $p.Y -ne $AtY) {
        throw "cursor at x=$($p.X) y=$($p.Y), wanted x=$AtX y=$AtY (input refused or point off screen)"
    }
}

function Read-CursorLine {
    $info = New-Object AkMouseInputW32+CURSORINFO
    $info.cbSize = [System.Runtime.InteropServices.Marshal]::SizeOf($info)
    if (-not [AkMouseInputW32]::GetCursorInfo([ref]$info)) {
        throw "GetCursorInfo failed (lasterr=$([System.Runtime.InteropServices.Marshal]::GetLastWin32Error()))"
    }
    $stock = [ordered]@{
        ARROW = 32512; IBEAM = 32513; WAIT = 32514; CROSS = 32515; UPARROW = 32516
        SIZENWSE = 32642; SIZENESW = 32643; SIZEWE = 32644; SIZENS = 32645; SIZEALL = 32646
        NO = 32648; HAND = 32649; APPSTARTING = 32650; HELP = 32651
    }
    $shape = 'custom'
    foreach ($name in $stock.Keys) {
        if ([AkMouseInputW32]::LoadCursorW([IntPtr]::Zero, [IntPtr]$stock[$name]) -eq $info.hCursor) { $shape = $name; break }
    }
    return ("ok cursor x=$px y=$py shape=$shape handle=0x{0:x}" -f $info.hCursor.ToInt64())
}

# ---- actions -----------------------------------------------------------------
$pressed = 0  # release flag still owed
try {
    switch ($Action) {
        'move' {
            Send-Mouse $px $py 0
            Assert-CursorAt $px $py
        }
        { $_ -in @('click', 'rightclick') } {
            $down = if ($Action -eq 'click') { $LEFTDOWN } else { $RIGHTDOWN }
            $up = if ($Action -eq 'click') { $LEFTUP } else { $RIGHTUP }
            Send-Mouse $px $py 0
            Assert-CursorAt $px $py
            $pressed = $up
            Send-Mouse $px $py $down
            Start-Sleep -Milliseconds 80
            Send-Mouse $px $py $up
            $pressed = 0
        }
        'drag' {
            Send-Mouse $px $py 0
            Assert-CursorAt $px $py
            $pressed = $LEFTUP
            Send-Mouse $px $py $LEFTDOWN
            Start-Sleep -Milliseconds 80
            for ($i = 1; $i -le $stepCount; $i++) {
                Send-Mouse ($px + [int](($px2 - $px) * $i / $stepCount)) ($py + [int](($py2 - $py) * $i / $stepCount)) 0
                Start-Sleep -Milliseconds 20
            }
            Assert-CursorAt $px2 $py2
            Send-Mouse $px2 $py2 $LEFTUP
            $pressed = 0
        }
        'shot' {
            Add-Type -AssemblyName System.Drawing
            $bmp = New-Object System.Drawing.Bitmap ($px2 - $px), ($py2 - $py)
            try {
                $g = [System.Drawing.Graphics]::FromImage($bmp)
                try { $g.CopyFromScreen($px, $py, 0, 0, $bmp.Size) } finally { $g.Dispose() }
                $bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
            } finally { $bmp.Dispose() }
        }
        'cursor' {
            Send-Mouse $px $py 0
            Assert-CursorAt $px $py
            if ($Press) {
                $pressed = $LEFTUP  # the catch below releases if the read throws
                Send-Mouse $px $py $LEFTDOWN
                Start-Sleep -Milliseconds 80
                Write-Output ((Read-CursorLine) + ' pressed=1')
                Send-Mouse $px $py $LEFTUP
                $pressed = 0
                Start-Sleep -Milliseconds 30
            }
            Exit-With 0 (Read-CursorLine)
        }
    }
} catch {
    $reason = $_.Exception.Message
    if ($pressed -ne 0) {
        # never leave a button held down
        $release = New-Object AkMouseInputW32+INPUT
        $release.type = 0
        $releaseMouse = New-Object AkMouseInputW32+MOUSEINPUT
        $releaseMouse.dwFlags = $pressed  # relative 0,0: release where the cursor is
        $release.mi = $releaseMouse
        $null = [AkMouseInputW32]::SendInput(1, @($release), [AkMouseInputW32]::InputSize())
    }
    Exit-With 1 "error $reason"
}
Exit-With 0 "ok $Action x=$px y=$py"
