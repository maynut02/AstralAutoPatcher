using Microsoft.Win32;
using System.Text.RegularExpressions;

namespace AstralAutoPatcher
{
  public static class SteamUtils
  {
    // 스팀 레지스트리 기본 경로
    private const string SteamRegistryPath = @"SOFTWARE\Valve\Steam";

    public static string? GetSteamInstallPath()
    {
      // 32비트 및 64비트 시스템 호환성을 위해 레지스트리에서 스팀 경로를 조회
      using var key = Registry.CurrentUser.OpenSubKey(SteamRegistryPath);
      if (key != null)
      {
        return key.GetValue("SteamPath")?.ToString()?.Replace("/", "\\");
      }

      return null;
    }

    public static List<string> GetLibraryPaths()
    {
      var paths = new List<string>();
      var steamPath = GetSteamInstallPath();

      if (string.IsNullOrEmpty(steamPath)) return paths;

      // 기본 스팀 설치 경로를 목록에 추가
      paths.Add(steamPath);

      // 추가 라이브러리 경로 찾기
      var vdfPath = Path.Combine(steamPath, "steamapps", "libraryfolders.vdf");
      if (File.Exists(vdfPath))
      {
        try
        {
          var content = File.ReadAllText(vdfPath);
          // 경로 정보를 추출
          var matches = Regex.Matches(content, "\"path\"\\s+\"(.+?)\"");

          foreach (Match match in matches)
          {
            if (match.Groups.Count > 1)
            {
              var path = match.Groups[1].Value.Replace("\\\\", "\\");
              if (!paths.Contains(path, StringComparer.OrdinalIgnoreCase))
              {
                paths.Add(path);
              }
            }
          }
        }
        catch (Exception ex)
        {
          System.Diagnostics.Debug.WriteLine($"libraryfolders.vdf 읽기 오류: {ex.Message}");
        }
      }

      return paths;
    }

    public const int GameAppId = 2622000;

    public static string? FindGameInstallPath()
    {
      var libraryPaths = GetLibraryPaths();

      foreach (var libPath in libraryPaths)
      {
        // 해당 게임의 appmanifest 파일 찾기
        var manifestPath = Path.Combine(libPath, "steamapps", $"appmanifest_{GameAppId}.acf");
        if (File.Exists(manifestPath))
        {
          try
          {
            var content = File.ReadAllText(manifestPath);
            // 설치 경로 정보 추출
            var match = Regex.Match(content, "\"installdir\"\\s+\"(.+?)\"");
            if (match.Success)
            {
              var installDirName = match.Groups[1].Value;
              var fullPath = Path.Combine(libPath, "steamapps", "common", installDirName);
              if (Directory.Exists(fullPath))
              {
                return fullPath;
              }
            }
          }
          catch
          {
            // 파일 읽기 실패 시 무시
          }
        }
      }

      return null;
    }
  }
}
