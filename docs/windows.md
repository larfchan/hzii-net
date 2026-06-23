# Windows startup

Place the files in a stable directory, for example:

```text
C:\Program Files\hzii-net\hzii-net.exe
C:\ProgramData\hzii-net\config.toml
```

When the startup task runs as `SYSTEM`, use an explicit shared state path in `config.toml`:

```toml
state_file = 'C:\ProgramData\hzii-net\session.json'
```

Restrict the plaintext credential file to administrators and `SYSTEM`:

```powershell
icacls "C:\ProgramData\hzii-net\config.toml" /inheritance:r /grant:r "SYSTEM:F" "Administrators:F"
```

Run PowerShell as Administrator:

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\packaging\windows\install-task.ps1 `
  -Binary "C:\Program Files\hzii-net\hzii-net.exe" `
  -Config "C:\ProgramData\hzii-net\config.toml"
```

This creates one startup task running `watch`. It does not periodically launch new login processes; keepalive and reconnection happen inside the running program.

Before running `logout`, stop the scheduled task. Otherwise the existing `watch` process will automatically log in again:

```powershell
Stop-ScheduledTask -TaskName hzii-net
& "C:\Program Files\hzii-net\hzii-net.exe" --config "C:\ProgramData\hzii-net\config.toml" logout
```

Remove it with:

```powershell
Unregister-ScheduledTask -TaskName hzii-net -Confirm:$false
```
