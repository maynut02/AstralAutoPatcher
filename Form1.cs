using System;
using System.ComponentModel;
using System.Windows.Forms;
using System.IO;
using System.IO.Compression;
using System.Threading.Tasks;

namespace AstralAutoPatch
{
  public partial class Form1 : Form
  {
    [DesignerSerializationVisibility(DesignerSerializationVisibility.Hidden)]
    public bool IsProtocolLaunch { get; set; } = false;

    // URL을 통해 전달받을 수 있는 게임 데이터 폴더명 (기본값 8vJXnINT)
    [DesignerSerializationVisibility(DesignerSerializationVisibility.Hidden)]
    public string TargetGameDataFolder { get; set; } = "8vJXnINT";


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
             restartArgs = $"astral://{TargetGameDataFolder}";
          }

          // .exe 파일을 다운로드하여 교체
          await UpdateManager.SelfUpdateAsync(latestAppRelease, progress, restartArgs);
          return;
        }

        // 3. 게임 폴더 내의 version.txt 내용과 깃허브의 최신 태그를 비교
        UpdateStatus("한글패치 버전을 확인 중입니다...");
        var latestPatchRelease = await UpdateManager.GetLatestReleaseAsync(UpdateManager.PatchRepoOwner, UpdateManager.PatchRepoName);
        await CheckAndPatchGameAsync(installPath, latestPatchRelease);

        UpdateStatus("모든 작업이 완료되었습니다. 창을 닫아 종료해주세요.");
      }
      catch (Exception ex)
      {
        MessageBox.Show($"오류 발생: {ex.Message}\n{ex.StackTrace}", "치명적 오류", MessageBoxButtons.OK, MessageBoxIcon.Error);
      }
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

      string clientVersion = "";
      string serverVersion = "";
      bool isNewInstall = false;

      if (File.Exists(clientVersionFilePath))
      {
        clientVersion = await File.ReadAllTextAsync(clientVersionFilePath);
      }
      
      if (File.Exists(serverVersionFilePath))
      {
        serverVersion = await File.ReadAllTextAsync(serverVersionFilePath);
      }

      if (string.IsNullOrWhiteSpace(clientVersion) && string.IsNullOrWhiteSpace(serverVersion))
      {
        isNewInstall = true;
        AddLog("설치된 한글패치가 없습니다.");
      }
      else
      {
        AddLog($"설치된 한글패치 버전 (Client): {clientVersion}");
        AddLog($"설치된 한글패치 버전 (Server): {serverVersion}");
      }

      AddLog($"최신 한글패치 버전: {release.TagName}");

      // 버전을 비교 (둘 중 하나라도 구버전이거나 없으면 업데이트)
      // if (isNewInstall || UpdateManager.IsNewerVersion(clientVersion, release.TagName) || 
      //     UpdateManager.IsNewerVersion(serverVersion, release.TagName))
      // {
      if (true)
      {
        UpdateStatus("한글패치 파일을 다운로드 중입니다...");

        // 패치 파일을 찾음
        var patchAsset = release.Assets.FirstOrDefault(a => a.Name.EndsWith(".zip", StringComparison.OrdinalIgnoreCase));
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

              // 루트 폴더 감지
              if (directories.Length == 1 && files.Length == 0)
              {
                rootPath = directories[0];
                AddLog($"루트 폴더 감지: {Path.GetFileName(rootPath)}");
              }

              // 1. AstralParty_INT_Data 덮어쓰기
              var sourceFolder1 = Path.Combine(rootPath, "AstralParty_INT_Data");
              if (Directory.Exists(sourceFolder1))
              {
                AddLog($"AstralParty_INT_Data를 복사하는 중입니다: {sourceFolder1}");
                var destFolder = Path.Combine(installPath, TargetGameDataFolder, "AstralParty_INT_Data");
                CopyDirectory(sourceFolder1, destFolder, true);
              }
              else
              {
                AddLog("경고: AstralParty_INT_Data 폴더를 찾을 수 없습니다.");
              }

              // 2. AstralParty_INT 덮어쓰기
              var sourceFolder2 = Path.Combine(rootPath, "AstralParty_INT");
              if (Directory.Exists(sourceFolder2))
              {
                var targetPath2 = Path.Combine(feimoPath, "AstralParty_INT");

                // 대상 폴더가 없으면 상위 폴더까지 생성
                Directory.CreateDirectory(targetPath2);

                AddLog($"AstralParty_INT를 복사하는 중입니다: {targetPath2}");
                CopyDirectory(sourceFolder2, targetPath2, true);
              }
              else
              {
                AddLog("경고: AstralParty_INT 폴더를 찾을 수 없습니다.");
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
      else
      {
        UpdateStatus("이미 최신 버전입니다.");
        await Task.Delay(1000);
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
