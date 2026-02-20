using System;
using System.ComponentModel;
using System.Windows.Forms;
using System.IO;
using System.IO.Compression;
using System.Threading.Tasks;

namespace AstralAutoPatcher
{
  public partial class Form1 : Form
  {
    [DesignerSerializationVisibility(DesignerSerializationVisibility.Hidden)]
    public bool IsProtocolLaunch { get; set; } = false;

    // 실행 모드: "patch" (기본값) 또는 "delete"
    [DesignerSerializationVisibility(DesignerSerializationVisibility.Hidden)]
    public string LaunchMode { get; set; } = "patch";

    public Form1()
    {
      InitializeComponent();
      lstLog.HorizontalScrollbar = true;
      this.Shown += Form1_Shown;
    }

    private async void Form1_Shown(object? sender, EventArgs e)
    {
      try
      {
        // UI 딜레이
        await Task.Delay(100);

        // 0. 게임 설치 경로 확인
        UpdateStatus("게임 설치 경로를 찾는 중입니다...");
        string? installPath = await Task.Run(() => SteamUtils.FindGameInstallPath());

        if (string.IsNullOrEmpty(installPath))
        {
          UpdateStatus("게임을 찾을 수 없습니다.");
          MessageBox.Show("게임을 찾을 수 없습니다. 게임이 설치되어 있는지 확인해주세요.", "오류", MessageBoxButtons.OK, MessageBoxIcon.Error);
          return;
        }

        // 0-1. 실행 파일 위치 확인 및 이동
        var currentExe = System.Diagnostics.Process.GetCurrentProcess().MainModule?.FileName;
        if (!string.IsNullOrEmpty(currentExe))
        {
          var currentDir = Path.GetDirectoryName(currentExe);
          // 경로를 정규화한 후 비교
          if (currentDir != null && !string.Equals(Path.GetFullPath(currentDir).TrimEnd('\\'), Path.GetFullPath(installPath).TrimEnd('\\'), StringComparison.OrdinalIgnoreCase))
          {
            UpdateStatus("게임 설치 폴더로 이동하는 중입니다...");
            AddLog($"현재 위치: {currentDir}");
            AddLog($"이동할 위치: {installPath}");

            var targetExe = Path.Combine(installPath, Path.GetFileName(currentExe));

            try
            {
              // 파일을 복사
              File.Copy(currentExe, targetExe, true);

              // 새 위치에서 프로그램을 실행
              var startInfo = new System.Diagnostics.ProcessStartInfo(targetExe)
              {
                UseShellExecute = true,
                WorkingDirectory = installPath
              };

              // 관리자 권한을 유지
              if (Program.IsAdministrator())
              {
                startInfo.Verb = "runas";
              }

              System.Diagnostics.Process.Start(startInfo);

              // 현재 프로세스를 종료
              Application.Exit();
              return;
            }
            catch (Exception ex)
            {
              MessageBox.Show($"이동 실패: {ex.Message}\n관리자 권한이 필요합니다.", "오류", MessageBoxButtons.OK, MessageBoxIcon.Error);
              Application.Exit();
              return;
            }
          }
        }

        // 1. 프로토콜 등록 (직접 실행 시에만 수행)
        if (!IsProtocolLaunch)
        {
          UpdateStatus("프로토콜 등록 상태를 확인 중입니다...");
          try
          {
            // 현재 위치가 올바른 위치이므로 등록을 진행
            ProtocolRegistrar.RegisterProtocol();
            AddLog("초기 설정이 완료되었습니다.");
            UpdateStatus("한글패치 사이트에서 [업데이트] 버튼을 눌러주세요.");
            AddLog("창을 닫아 종료해주세요.");
            // 직접 실행 시에는 업데이트 로직을 수행하지 않고 종료
            return;
          }
          catch (Exception ex)
          {
            MessageBox.Show($"프로토콜 등록 실패: {ex.Message}", "오류", MessageBoxButtons.OK, MessageBoxIcon.Error);
          }
        }

        // 2. 현재 프로그램의 버전과 깃허브의 최신 태그를 비교
        UpdateStatus("프로그램 버전을 확인 중입니다...");
        var latestAppRelease = await UpdateManager.GetLatestReleaseAsync(UpdateManager.AppRepoOwner, UpdateManager.AppRepoName);
        if (latestAppRelease != null && UpdateManager.IsNewerVersion(UpdateManager.CurrentVersion, latestAppRelease.TagName))
        {
          UpdateStatus("프로그램의 새 버전을 발견했습니다! 업데이트를 진행합니다...");
          
          var progress = new Progress<int>(percent =>
          {
            if (progressBar1.InvokeRequired)
              progressBar1.Invoke(new Action(() => progressBar1.Value = percent));
            else
              progressBar1.Value = percent;
          });

          // 재시작 시 전달할 인자 구성
          string restartArgs = "";
          if (IsProtocolLaunch)
          {
             // 업데이트 후 재시작 시에도 원래 실행 모드(patch/delete)를 유지
             restartArgs = $"astral://{LaunchMode}";
          }

          // .exe 파일을 다운로드하여 교체
          await UpdateManager.SelfUpdateAsync(latestAppRelease, progress, restartArgs);
          return;
        }

        // 3. 실행 모드에 따른 분기 처리
        if (LaunchMode == "delete")
        {
          UpdateStatus("한글패치를 삭제하는 중입니다...");
          await DeletePatchAsync(installPath);
        }
        else // 기본값 "patch"
        {
          UpdateStatus("한글패치 버전을 확인 중입니다...");
          var latestPatchRelease = await UpdateManager.GetLatestReleaseAsync(UpdateManager.PatchRepoOwner, UpdateManager.PatchRepoName);
          await CheckAndPatchGameAsync(installPath, latestPatchRelease);
        }

        UpdateStatus("모든 작업이 완료되었습니다. 창을 닫아 종료해주세요.");
      }
      catch (Exception ex)
      {
        MessageBox.Show($"오류 발생: {ex.Message}\n{ex.StackTrace}", "치명적 오류", MessageBoxButtons.OK, MessageBoxIcon.Error);
      }
    }

    private async Task DeletePatchAsync(string installPath)
    {
      await Task.Run(() =>
      {
        try
        {
          // 1. feimo 패치 폴더 삭제
          var userProfile = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
          var feimoPatchPath = Path.Combine(userProfile, "AppData", "LocalLow", "feimo", "AstralParty_INT", "com.unity.addressables", "AssetBundles");
          
          if (Directory.Exists(feimoPatchPath))
          {
            AddLog($"패치 폴더 삭제 중: {feimoPatchPath}");
            Directory.Delete(feimoPatchPath, true);
            AddLog("feimo 패치 파일이 삭제되었습니다.");
          }
          else
          {
            AddLog("삭제할 feimo 패치 폴더가 없습니다.");
          }

          // 2. 8vJXnINT 폴더 삭제 (게임 설치 경로 내)
          // 주의: 8vJXnINT 폴더 전체를 삭제할지, 혹은 내부의 특정 파일만 삭제할지 결정 필요.
          // 여기서는 패치로 추가된 8vJXnINT 폴더 전체를 삭제하는 것으로 가정합니다.
          // 만약 8vJXnINT가 게임 원본 데이터라면 삭제하면 안 됩니다. 
          // 하지만 이전 로직에서 8vJXnINT를 통째로 덮어씌웠으므로, 패치 삭제 시 해당 폴더를 지우는 것이 맞을 수 있습니다.
          // 사용자의 요청은 "한글패치 삭제"이므로, 패치로 인해 변경된 사항을 되돌리는 것이 목표입니다.
          // 8vJXnINT 폴더가 패치로만 생성되는 폴더인지, 원본 게임에도 존재하는지 확인이 필요하지만,
          // 요청하신 내용은 "feimo/.../AssetBundles 폴더를 삭제하게 만들어줘" 였으므로, 
          // 명시된 feimo 폴더 삭제 외에 게임 설치 경로 쪽 파일 처리에 대한 명시적 언급은 없었습니다.
          // 그러나 "한글패치 삭제"라는 맥락상, 게임 설치 폴더에 복사된 8vJXnINT 파일들도 처리하는 것이 좋습니다.
          // 일단 요청하신 대로 feimo 쪽 AssetBundles 삭제는 구현했고, 
          // 게임 설치 폴더 쪽(8vJXnINT)은 원본 손상 위험이 있으므로 건드리지 않거나, 
          // 명확한 지시가 없으므로 feimo 쪽만 삭제하도록 하겠습니다.
          
          // (추가) 요청 사항 재확인: "delete는 AppData/.../AssetBundles 폴더를 삭제하게 만들어줘.(한글패치 삭제)"
          // 따라서 feimo 쪽만 삭제합니다.

        }
        catch (Exception ex)
        {
          AddLog($"삭제 중 오류 발생: {ex.Message}");
          MessageBox.Show($"삭제 실패: {ex.Message}", "오류", MessageBoxButtons.OK, MessageBoxIcon.Error);
        }
      });
    }

    private async Task CheckAndPatchGameAsync(string installPath, GitHubRelease? release)
    {
      if (release == null) return;

      // 로컬 버전을 확인 (Client)
      var clientVersionFilePath = Path.Combine(installPath, "version.txt");
      
      // 로컬 버전을 확인 (Server - AppData/feimo)
      var userProfile = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
      var feimoPath = Path.Combine(userProfile, "AppData", "LocalLow", "feimo");
      var serverVersionFilePath = Path.Combine(feimoPath, "version.txt");

      AddLog($"최신 한글패치 버전: {release.TagName}");

      UpdateStatus("한글패치 파일을 다운로드 중입니다...");

      // 패치 파일을 찾음 (Steam-EN 버전 우선 선택)
      var patchAsset = release.Assets.FirstOrDefault(a => 
        a.Name.StartsWith("AstralParty-KoPatch-Steam-EN-", StringComparison.OrdinalIgnoreCase) && 
        a.Name.EndsWith(".zip", StringComparison.OrdinalIgnoreCase));

      if (patchAsset != null)
      {
        AddLog($"패치 파일 발견: {patchAsset.Name}");
        var tempZipPath = Path.Combine(Path.GetTempPath(), patchAsset.Name);
        var extractPath = Path.Combine(Path.GetTempPath(), "AstralPatch_Extract");

        var progress = new Progress<int>(percent =>
        {
          if (progressBar1.InvokeRequired)
            progressBar1.Invoke(new Action(() => progressBar1.Value = percent));
          else
            progressBar1.Value = percent;
        });

        await UpdateManager.DownloadFileAsync(patchAsset.BrowserDownloadUrl, tempZipPath, progress);

        UpdateStatus("한글패치를 적용하는 중입니다...");
        await Task.Run(() =>
        {
          try
          {
            // 임시 폴더를 초기화
            if (Directory.Exists(extractPath)) Directory.Delete(extractPath, true);
            Directory.CreateDirectory(extractPath);

            AddLog("압축을 해제하는 중입니다...");
            ZipFile.ExtractToDirectory(tempZipPath, extractPath);

            // 압축 해제된 폴더 구조를 확인
            string rootPath = extractPath;
            var directories = Directory.GetDirectories(extractPath);
            var files = Directory.GetFiles(extractPath);

            // 루트 폴더 감지 (폴더가 하나만 있고 파일이 없으면 진입)
            if (directories.Length == 1 && files.Length == 0)
            {
              rootPath = directories[0];
              AddLog($"루트 폴더 감지: {Path.GetFileName(rootPath)}");
            }

            // // 1. feimo 폴더 처리 (AppData/LocalLow/feimo)
            // var sourceFeimo = Path.Combine(rootPath, "feimo");
            // if (Directory.Exists(sourceFeimo))
            // {
            //   AddLog($"feimo 폴더를 업데이트하는 중입니다...");
            //   // feimoPath는 이미 AppData/LocalLow/feimo 를 가리킴
            //   if (!Directory.Exists(feimoPath)) Directory.CreateDirectory(feimoPath);
            //   CopyDirectory(sourceFeimo, feimoPath, true);
            // }
            // else
            // {
            //   AddLog("경고: 압축 파일 내에 feimo 폴더가 없습니다.");
            // }

            // 1. AstralParty_INT 폴더 처리 (AppData/LocalLow/feimo/AstralParty_INT)
            var sourceAstral = Path.Combine(rootPath, "AstralParty_INT");
            if (Directory.Exists(sourceAstral))
            {
              AddLog($"AstralParty_INT 폴더를 업데이트하는 중입니다...");
              var destAstral = Path.Combine(feimoPath, "AstralParty_INT");
              if (!Directory.Exists(destAstral)) Directory.CreateDirectory(destAstral);
              CopyDirectory(sourceAstral, destAstral, true);
            }
            else
            {
              AddLog("경고: 압축 파일 내에 AstralParty_INT 폴더가 없습니다.");
            }

            // 2. 8vJXnINT 폴더 처리 (GameInstallPath/8vJXnINT)
            var sourceGameData = Path.Combine(rootPath, "8vJXnINT");
            if (Directory.Exists(sourceGameData))
            {
              var destGameData = Path.Combine(installPath, "8vJXnINT");
              AddLog($"8vJXnINT 폴더를 업데이트하는 중입니다...");
              CopyDirectory(sourceGameData, destGameData, true);
            }
            else
            {
              AddLog("경고: 압축 파일 내에 8vJXnINT 폴더가 없습니다.");
            }

            // 3. version.txt 덮어쓰기
            var sourceVersionFile = Path.Combine(rootPath, "version.txt");
            
            // feimo 폴더 생성 확인
            if (!Directory.Exists(feimoPath)) Directory.CreateDirectory(feimoPath);

            if (File.Exists(sourceVersionFile))
            {
              File.Copy(sourceVersionFile, clientVersionFilePath, true);
              File.Copy(sourceVersionFile, serverVersionFilePath, true);
            }
            else
            {
              // zip 안에 version.txt가 없으면 태그 이름으로 생성
              File.WriteAllText(clientVersionFilePath, release.TagName);
              File.WriteAllText(serverVersionFilePath, release.TagName);
              AddLog("version.txt 생성 완료");
            }

            AddLog("한글패치 적용이 완료되었습니다.");
          }
          catch (Exception ex)
          {
            AddLog($"오류 발생: {ex.Message}");
            MessageBox.Show($"패치 적용 실패: {ex.Message}", "오류", MessageBoxButtons.OK, MessageBoxIcon.Error);
          }
          finally
          {
            if (File.Exists(tempZipPath)) File.Delete(tempZipPath);
            if (Directory.Exists(extractPath)) Directory.Delete(extractPath, true);
          }
        });
      }
      else
      {
        AddLog("릴리즈에 .zip 파일이 없습니다.");
      }
    }

    private void CopyDirectory(string sourceDir, string destinationDir, bool recursive)
    {
      // 원본 디렉토리 정보를 가져옴
      var dir = new DirectoryInfo(sourceDir);
      if (!dir.Exists) throw new DirectoryNotFoundException($"원본 디렉토리를 찾을 수 없습니다: {dir.FullName}");

      DirectoryInfo[] dirs = dir.GetDirectories();
      // 대상 디렉토리가 없으면 생성
      Directory.CreateDirectory(destinationDir);

      // 파일들을 복사
      foreach (FileInfo file in dir.GetFiles())
      {
        string targetFilePath = Path.Combine(destinationDir, file.Name);
        file.CopyTo(targetFilePath, true);
      }

      // 하위 디렉토리도 재귀적으로 복사
      if (recursive)
      {
        foreach (DirectoryInfo subDir in dirs)
        {
          string newDestinationDir = Path.Combine(destinationDir, subDir.Name);
          CopyDirectory(subDir.FullName, newDestinationDir, true);
        }
      }
    }

    private void AddLog(string message)
    {
      // 로그 추가
      if (lstLog.InvokeRequired)
      {
        lstLog.Invoke(new Action<string>(AddLog), message);
      }
      else
      {
        string timestamp = DateTime.Now.ToString("HH:mm:ss");
        lstLog.Items.Add($"[{timestamp}] {message}");
        // 최신 로그가 보이도록 스크롤을 아래로 이동
        lstLog.TopIndex = lstLog.Items.Count - 1;
      }
    }

    private void UpdateStatus(string message)
    {
      // 상태 메시지 업데이트
      if (lblStatus.InvokeRequired)
      {
        lblStatus.Invoke(new Action<string>(UpdateStatus), message);
      }
      else
      {
        lblStatus.Text = message;
        // 상태 변경 시 로그에도 함께 기록
        AddLog(message);
      }
    }
  }
}
