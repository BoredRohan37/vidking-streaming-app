MovieBox-TUI  -  Windows 7 / 8.1 edition
=========================================

QUICK START
  1. Put mpv.exe (and its DLLs/folder) into the "tools" folder.
     See tools\README.txt for details. yt-dlp.exe and ffmpeg.exe are only
     needed for downloads.
  2. Double-click run.bat.

KEYS
  /            search
  Enter        play / select
  Ctrl+L       network log
  q or Esc     quit

WHAT IS DIFFERENT FROM THE NORMAL VERSION
  - Built for Windows 7 SP1 and Windows 8.1 (not for Windows 10/11 - use the
    normal release there).
  - Looks for mpv / VLC / yt-dlp / ffmpeg in the "tools" folder first.
  - Auto-update is turned off. Download new versions manually.
  - The classic Windows console cannot show true colors or poster images, so
    this edition automatically uses a simple 16-color layout there.

IF THE SCREEN LOOKS WRONG
  - Right-click the title bar -> Properties -> Font -> choose "Lucida Console"
    or "Consolas".
  - For a nicer look, run run.bat inside ConEmu or Cmder.
  - Set the environment variable MOVIEBOX_LEGACY_CONSOLE=1 to force the simple
    layout, or MOVIEBOX_LEGACY_CONSOLE=0 to force the full layout.
  - Set MOVIEBOX_NO_IMAGE=1 to turn off poster images.

LOGS
  Press Ctrl+L inside the app, or check the log folder shown in the app's
  Settings.
