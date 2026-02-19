using System.Text.Json;
using System.Text.Json.Serialization;
using System.Diagnostics;
using System.IO.Compression;
using System.Net.Http.Headers;

namespace AstralAutoPatcher
{
  public class GitHubRelease
  {
    [JsonPropertyName("tag_name")]
    public string TagName { get; set; } = "";

    [JsonPropertyName("assets")]
    public List<GitHubAsset> Assets { get; set; } = new();
  }

  public class GitHubAsset
  {
    [JsonPropertyName("name")]
    public string Name { get; set; } = "";

    [JsonPropertyName("browser_download_url")]
    public string BrowserDownloadUrl { get; set; } = "";
  }

  public static class UpdateManager
  {
    // 프로그램 실행 파일이 저장된 리포지토리
    public const string AppRepoOwner = "maynut02";
    public const string AppRepoName = "AstralAutoPatcher";

    // 한글 패치 파일이 저장된 리포지토리
    public const string PatchRepoOwner = "maynut02";
    public const string PatchRepoName = "AstralParty-KoPatch";

    // 현재 프로그램 버전
    public static string CurrentVersion 
    {
        get
        {
            var v = System.Reflection.Assembly.GetExecutingAssembly().GetName().Version;
            // v1.0.0 형식으로 반환
            return v != null ? $"v{v.Major}.{v.Minor}.{v.Build}" : "v1.0.0";
        }
    }

    private static readonly HttpClient _client = new HttpClient();

    static UpdateManager()
    {
      _client.DefaultRequestHeaders.UserAgent.Add(new ProductInfoHeaderValue("AstralAutoPatcher", "1.0"));
    }

    public static async Task<GitHubRelease?> GetLatestReleaseAsync(string owner, string repo)
    {
      try
      {
        // GitHub API를 호출하여 최신 릴리즈 정보를 가져옴
        var url = $"https://api.github.com/repos/{owner}/{repo}/releases/latest";
        var response = await _client.GetStringAsync(url);
        return JsonSerializer.Deserialize<GitHubRelease>(response);
      }
      catch (Exception ex)
      {
        Debug.WriteLine($"최신 릴리즈 정보 조회 실패: {ex.Message}");
        return null;
      }
    }

    public static async Task DownloadFileAsync(string url, string destinationPath, IProgress<int> progress)
    {
      using var response = await _client.GetAsync(url, HttpCompletionOption.ResponseHeadersRead);
      response.EnsureSuccessStatusCode();

      var totalBytes = response.Content.Headers.ContentLength ?? -1L;
      var canReportProgress = totalBytes != -1 && progress != null;

      using var contentStream = await response.Content.ReadAsStreamAsync();
      using var fileStream = new FileStream(destinationPath, FileMode.Create, FileAccess.Write, FileShare.None, 8192, true);

      var buffer = new byte[8192];
      long totalRead = 0;
      int bytesRead;

      while ((bytesRead = await contentStream.ReadAsync(buffer, 0, buffer.Length)) > 0)
      {
        await fileStream.WriteAsync(buffer, 0, bytesRead);
        totalRead += bytesRead;

        if (canReportProgress)
        {
          progress.Report((int)((double)totalRead / totalBytes * 100));
        }
      }
    }

    public static async Task SelfUpdateAsync(GitHubRelease release, IProgress<int> progress, string restartArguments = "")
    {
      // 현재 실행 중인 파일의 경로를 가져옴
      var currentExe = Process.GetCurrentProcess().MainModule?.FileName;
      if (string.IsNullOrEmpty(currentExe)) return;

      if (release.Assets == null) return;

      // .exe 파일을 찾기
      var asset = release.Assets.FirstOrDefault(a => a.Name.EndsWith(".exe", StringComparison.OrdinalIgnoreCase));
      if (asset == null) return;

      var tempPath = currentExe + ".new";

      // 다운로드
      await DownloadFileAsync(asset.BrowserDownloadUrl, tempPath, progress);

      // 배치 파일을 생성하여 교체 및 재시작 수행
      var batchPath = Path.Combine(Path.GetDirectoryName(currentExe)!, "update.bat");
      var batchContent = $@"
@echo off
timeout /t 1 /nobreak > NUL
del ""{currentExe}""
move ""{tempPath}"" ""{currentExe}""
start """" ""{currentExe}"" {restartArguments}
del ""%~f0""
";
      await File.WriteAllTextAsync(batchPath, batchContent);

      // 배치 파일 실행 및 현재 프로그램 종료
      var psi = new ProcessStartInfo
      {
        FileName = batchPath,
        CreateNoWindow = true,
        UseShellExecute = false
      };
      Process.Start(psi);
      Environment.Exit(0);
    }

    public static bool IsNewerVersion(string localVersion, string remoteVersion)
    {
      // 버전 문자열이 비어있으면 업데이트 필요
      if (string.IsNullOrWhiteSpace(localVersion)) return true;
      if (string.IsNullOrWhiteSpace(remoteVersion)) return false;

      // 단순 문자열 비교 (다르면 업데이트)
      return !string.Equals(localVersion.Trim(), remoteVersion.Trim(), StringComparison.OrdinalIgnoreCase);
    }
  }
}
