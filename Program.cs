using System;
using System.Diagnostics;
using System.Security.Principal;
using System.Windows.Forms;

namespace AstralAutoPatch
{
  internal static class Program
  {
    [STAThread]
    static void Main(string[] args)
    {
      // .NET 6+ WinForms 초기화
      ApplicationConfiguration.Initialize();

      var form = new Form1();

      // 실행 인자 처리
      if (args.Length > 0)
      {
        // astral:// 프로토콜을 통해 실행된 경우, 첫 번째 인자로 전체 URI가 전달
        string uriString = args[0];
        if (uriString.StartsWith("astral://", StringComparison.OrdinalIgnoreCase))
        {
          form.IsProtocolLaunch = true;
          // URI의 Host 부분을 실행 모드로 사용 (예: astral://patch 또는 astral://delete)
          try 
          {
             var uri = new Uri(uriString);
             if (!string.IsNullOrEmpty(uri.Host))
             {
               form.LaunchMode = uri.Host.ToLower();
             }
          }
          catch { }
        }
      }

      Application.Run(form);
    }

    public static bool IsAdministrator()
    {
      using (var identity = WindowsIdentity.GetCurrent())
      {
        var principal = new WindowsPrincipal(identity);
        return principal.IsInRole(WindowsBuiltInRole.Administrator);
      }
    }
  }
}
