using Microsoft.Win32;
using System.Diagnostics;
using System.Reflection;

namespace AstralAutoPatcher
{
  public static class ProtocolRegistrar
  {
    private const string ProtocolName = "astral";

    public static bool IsRegistered()
    {
      using var key = Registry.ClassesRoot.OpenSubKey(ProtocolName);
      return key != null;
    }

    public static void RegisterProtocol()
    {
      try
      {
        var exePath = Process.GetCurrentProcess().MainModule?.FileName;
        if (string.IsNullOrEmpty(exePath)) return;

        // HKEY_CLASSES_ROOT\astral 키를 생성
        using var key = Registry.ClassesRoot.CreateSubKey(ProtocolName);
        key.SetValue("", "URL:Astral Protocol");
        key.SetValue("URL Protocol", "");

        // 쉘 실행 명령을 설정
        using var commandKey = key.CreateSubKey(@"shell\open\command");
        commandKey.SetValue("", $"\"{exePath}\" \"%1\"");
      }
      catch (UnauthorizedAccessException)
      {
        throw new Exception("프로토콜 등록을 위해서는 관리자 권한이 필요합니다.");
      }
    }

    public static void UnregisterProtocol()
    {
      try
      {
        Registry.ClassesRoot.DeleteSubKeyTree(ProtocolName, false);
      }
      catch (UnauthorizedAccessException)
      {
        throw new Exception("프로토콜 해제를 위해서는 관리자 권한이 필요합니다.");
      }
    }
  }
}
